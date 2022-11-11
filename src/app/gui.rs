use std::{env, thread};
use std::iter::FromIterator;
use std::option::Option::Some;
use std::process::exit;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use env_logger::Env;
use futures::lock::Mutex;
use log::error;
use path_absolutize::Absolutize;
use sciter::Value;
use sysinfo::SystemExt;
use tokio::runtime::Runtime;
use tokio::task;
use tokio::task::JoinHandle;

use crate::app::api::LauncherApi;
use crate::{LAUNCHER_DIRECTORY, LauncherOptions};
use crate::minecraft::launcher::{LauncherData, LaunchingParameter};
use crate::minecraft::{prelauncher, service};
use crate::minecraft::progress::ProgressUpdate;
use crate::minecraft::service::{Account, auth_msa, auth_offline, authenticate_mojang};

struct RunnerInstance {
    terminator: tokio::sync::oneshot::Sender<()>,
}

struct ConstantLauncherData {
}

struct EventHandler {
    constant_data: Arc<ConstantLauncherData>,
    runner_instance: Arc<Mutex<Option<RunnerInstance>>>,
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    async_runtime: Runtime,
}

struct EventFunctions {
    on_output: Value,
    on_progress: Value,
}

fn handle_stdout(value: &Arc<std::sync::Mutex<EventFunctions>>, data: &[u8]) -> Result<()> {
    value.lock().unwrap().on_output.call(None, &make_args!("stdout", String::from_utf8(data.to_vec()).unwrap()), None)?;

    Ok(())
}

fn handle_stderr(value: &Arc<std::sync::Mutex<EventFunctions>>, data: &[u8]) -> Result<()> {
    value.lock().unwrap().on_output.call(None, &make_args!("stderr", String::from_utf8(data.to_vec()).unwrap()), None)?;

    Ok(())
}

fn handle_progress(value: &Arc<std::sync::Mutex<EventFunctions>>, progress_update: ProgressUpdate) -> Result<()> {
    let funcs = value.lock().unwrap();

    match progress_update {
        ProgressUpdate::SetMax(max) => funcs.on_progress.call(None, &make_args!("max", max as i32), None),
        ProgressUpdate::SetProgress(progress) => funcs.on_progress.call(None, &make_args!("progress", progress as i32), None),
        ProgressUpdate::SetLabel(label) => funcs.on_progress.call(None, &make_args!("label", label), None)
    }?;

    Ok(())
}

impl EventHandler {

    // script handler
    fn run_client(&self, build_id: i32, account_data: Value, options: Value, on_progress: Value, on_output: Value, on_finalization: Value, on_error: Value) -> bool {
        let account = serde_json::from_str::<Account>(&account_data.to_string()).unwrap();
        let options = LauncherOptions::from_json(options.to_string()).unwrap();

        let (account_name, uuid, token, user_type) = match account {
            Account::MsaAccount { auth, .. } => (auth.name, auth.uuid, auth.token, "msa".to_string()),
            Account::MojangAccount { name, token, uuid } => (name, token, uuid, "mojang".to_string()),
            Account::OfflineAccount { name, uuid } => (name, "-".to_string(), uuid, "legacy".to_string())
        };

        let runner_instance_clone = self.runner_instance.clone();

        let mut runner_instance_content = self.async_runtime.block_on(self.runner_instance.lock());
        let mut join_handle = self.async_runtime.block_on(self.join_handle.lock());

        if runner_instance_content.is_some() {
            return true;
        }

        let (terminator_tx, terminator_rx) = tokio::sync::oneshot::channel();

        let sys = sysinfo::System::new_all();
        let parameters = LaunchingParameter {
            memory: ((sys.total_memory() / 1000000) as f64 * (options.memory_percentage as f64 / 100.0)) as i64,
            custom_java_path: if !options.custom_java_path.is_empty() { Some(options.custom_java_path) } else { None },
            auth_player_name: account_name,
            auth_uuid: uuid,
            auth_access_token: token,
            auth_xuid: "x".to_string(),
            clientid: service::AZURE_CLIENT_ID.to_string(),
            user_type,
            keep_launcher_open: options.keep_launcher_open
        };

        let jh = self.async_runtime.spawn(async move {
            // todo: cache builds somewhere
            let builds = match LauncherApi::load_all_builds().await {
                Ok(build) => build,
                Err(err) => {
                    on_error.call(None, &make_args!(err.to_string()), None).unwrap();
                    return;
                }
            };
            let build = match builds.iter().find(|x| x.build_id == build_id as u32) {
                Some(build) => build,
                None => {
                    on_error.call(None, &make_args!("unable to find build"), None).unwrap();
                    return;
                }
            };

            if let Err(err) = prelauncher::launch(
                build,
                parameters,
                LauncherData {
                    on_stdout: handle_stdout,
                    on_stderr: handle_stderr,
                    on_progress: handle_progress,
                    data: Box::new(Arc::new(std::sync::Mutex::new(EventFunctions { on_output, on_progress }))),
                    terminator: terminator_rx
                }
            ).await {
                on_error.call(None, &make_args!(err.to_string()), None).unwrap();
            }

            { *runner_instance_clone.lock().await = None; }

            on_finalization.call(None, &make_args!(), None).unwrap();
        });

        *runner_instance_content = Some(RunnerInstance { terminator: terminator_tx });
        *join_handle = Some(jh);

        true
    }

    fn terminate(&self) -> bool {
        let runner_instance = self.runner_instance.clone();
        let join_handle = self.join_handle.clone();

        self.async_runtime.spawn(async move {
            {
                let mut lck = runner_instance.lock().await;

                if let Some(inst) = lck.take() {
                    println!("Sending sigterm");
                    inst.terminator.send(()).unwrap();
                }
            }

            join_handle.lock().await.take().unwrap().await.unwrap();
        });

        true
    }

    // script handler
    fn get_branches(&self, on_response: Value, on_error: Value) -> bool {
        self.async_runtime.spawn(async move {
            match LauncherApi::load_branches().await {
                Ok(branches) => {
                    on_response.call(None, &make_args!(Value::from_iter(branches)), None).unwrap()
                },
                Err(err) => {
                    error!("{:?}", err);

                    on_error.call(None, &make_args!(err.to_string()), None).unwrap()
                }
            };
        });

        true
    }

    fn get_builds(&self, branch: String, on_response: Value, on_error: Value) -> bool {
        self.async_runtime.spawn(async move {
            match LauncherApi::load_builds(branch).await {
                Ok(builds) => {
                    let builds = Value::from_iter(builds.iter().map(|x| {
                        Value::parse(&*serde_json::to_string(x).unwrap()).unwrap()
                    }).collect::<Vec<Value>>());

                    on_response.call(None, &make_args!(builds), None).unwrap()
                },
                Err(err) => {
                    error!("{:?}", err);

                    on_error.call(None, &make_args!(err.to_string()), None).unwrap()
                }
            };
        });

        true
    }

    fn login_offline(&self, username: String, on_response: Value) -> bool {
        self.async_runtime.spawn(async move {
            let acc = auth_offline(username).await;
            on_response.call(None, &make_args!(Value::parse(&*serde_json::to_string(&acc).unwrap()).unwrap()), None).unwrap();
        });

        true
    }

    fn login_msa(&self, on_error: Value, on_code: Value, on_response: Value) -> bool {
        // todo: fork library and make it async
        thread::spawn(move || {
            let on_code_fn = |code: &String| {
                on_code.call(None, &make_args!(Value::parse(&*code).unwrap()), None).unwrap();
            };

            match auth_msa(on_code_fn) {
                Ok(acc) => {
                    on_response.call(None, &make_args!(Value::parse(&*serde_json::to_string(&acc).unwrap()).unwrap()), None).unwrap()
                },
                Err(err) => {
                    println!("{:?}", err);

                    on_error.call(None, &make_args!(err.to_string()), None).unwrap()
                }
            };
        });

        true
    }

    fn login_mojang(&self, username: String, password: String, on_error: Value, on_response: Value) -> bool {
        self.async_runtime.spawn(async move {
            match authenticate_mojang(username, password).await {
                Ok(acc) => {
                    on_response.call(None, &make_args!(Value::parse(&*serde_json::to_string(&acc).unwrap()).unwrap()), None).unwrap()
                },
                Err(err) => {
                    println!("{:?}", err);

                    on_error.call(None, &make_args!(err.to_string()), None).unwrap()
                }
            };
        });

        true
    }

    fn logout(&self, account_data: Value) {
        self.async_runtime.spawn(async move {
            let acc = serde_json::from_str::<Account>(&*account_data.to_string()).unwrap();
            let _ = acc.logout().await; // we don't care if logouts fails...
        });

    }

    fn get_options(&self) -> Value {
        let config_dir = LAUNCHER_DIRECTORY.config_dir();
        let options = LauncherOptions::load(config_dir).unwrap_or_default(); // default to basic options if unable to load
        let json_options = options.to_json().unwrap();

        Value::parse(&*json_options).unwrap()
    }

    fn store_options(&self, options: Value) -> bool {
        let config_dir = LAUNCHER_DIRECTORY.config_dir();
        match LauncherOptions::from_json(options.to_string()) {
            Ok(launcher_options) => launcher_options.store(config_dir).unwrap(),
            Err(e) => error!("Storing options failed due to {}", e)
        };

        true
    }

    ///
    /// This will run an update check in the background asynchronous to the sciter frame until it is
    /// completed. If an update has been found the required arguments will be passed to the function.
    /// If not or a error has appeared it will be logged to the console, but without any user notice.
    /// A error is unlikely to be the users fault, so it is not something we won't them to know.
    ///
    fn check_for_updates(&self, found_newer_version: Value) -> bool {
        self.async_runtime.spawn(async move {
            match crate::updater::compare_versions().await {
                Ok((is_it_newer_version, newest_version)) => {
                    if !is_it_newer_version {
                        // there is no newer version
                        return
                    }

                    // Call out newer version found function to Sciter JS and pass github release data
                    found_newer_version.call(None, &make_args!(Value::parse(&*serde_json::to_string(&newest_version).unwrap()).unwrap()), None).unwrap();
                }
                Err(e) => error!("Update check failed {}", e)
            }
        });

        true
    }

    fn open(&self, url: String) -> bool {
        open::that(url).unwrap();

        true
    }

    fn exit_app(&self) {
        // exit app
        exit(0);
    } 

}

impl sciter::EventHandler for EventHandler {
    fn get_subscription(&mut self) -> Option<sciter::dom::event::EVENT_GROUPS> {
        Some(sciter::dom::event::default_events() | sciter::dom::event::EVENT_GROUPS::HANDLE_METHOD_CALL)
    }

    // route script calls to our handler
    dispatch_script_call! {
		fn run_client(i32, Value, Value, Value, Value, Value, Value);
		fn terminate();
        fn get_options();
        fn store_options(Value);
		fn get_branches(Value, Value);
        fn get_builds(String, Value, Value);
        fn login_offline(String, Value);
        fn login_msa(Value, Value, Value);
        fn login_mojang(String, String, Value, Value);
        fn logout(Value);
        fn check_for_updates(Value);
        fn open(String);
        fn exit_app();
	}
}


/// Runs the GUI and returns when the window is closed.
pub(crate) fn gui_main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let gui_index = get_path().unwrap();

    let mut frame = sciter::WindowBuilder::main_window()
        .glassy()
        .alpha()
        .fixed()
        .debug()
        .with_size((1000, 600))
        .create();

    frame.event_handler(EventHandler { constant_data: Arc::new(ConstantLauncherData { }), runner_instance: Arc::new(Mutex::new(None)), join_handle: Arc::new(Default::default()), async_runtime: Runtime::new().unwrap() });

    frame.load_file(&gui_index);
    frame.run_app();
}

fn get_path() -> Result<String> {
    let mut app_path = env::current_dir()?;
    app_path.push("app");

    let local_index = if app_path.join("public").exists() { // useful for dev env
        app_path.join("public")
    } else {
        app_path
    }.join("index.html");

    if !local_index.exists() {
        return Err(anyhow!("unable to find app index"));
    }

    let absolut_path = local_index.absolutize()?;
    return Ok(format!("file://{}", absolut_path.to_str().unwrap_or("index.html")));
}