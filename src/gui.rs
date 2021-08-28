use path_absolutize::Absolutize;
use sciter::Value;
use sciter::window::Options;
use std::env;
use std::option::Option::Some;
use std::path::PathBuf;
use sciter::dom::event::{default_events, EVENT_GROUPS};
use std::iter::FromIterator;
use crate::cloud::{ClientVersionManifest, SUPPORTED_CLOUD_FILE_VERSION};
use crate::minecraft::service::{Account, AuthService};
use std::sync::Arc;
use futures::lock::{Mutex, MutexGuard};
use tokio::runtime::Runtime;
use tokio::task;
use crate::minecraft::version::VersionManifest;
use crate::minecraft::launcher::{LauncherData, ProgressUpdate};
use anyhow::{Error, Result};
use std::borrow::Borrow;

struct RunnerInstance {
    terminator: tokio::sync::oneshot::Sender<()>,
}

struct ConstantLauncherData {
    version_manifest: VersionManifest,
    client_version_manifest: ClientVersionManifest,
}

struct EventHandler {
    constant_data: Arc<ConstantLauncherData>,
    runner_instance: Arc<Mutex<Option<RunnerInstance>>>,
    join_handle: Arc<Mutex<Option<task::JoinHandle<()>>>>,
    async_runtime: Runtime,
}

struct EventFunctions {
    on_output: Value,
    on_progress: Value,
}

fn handle_stdout(value: &Arc<std::sync::Mutex<EventFunctions>>, data: &[u8]) -> anyhow::Result<()> {
    value.lock().unwrap().on_output.call(None, &make_args!("stdout", String::from_utf8(data.to_vec()).unwrap()), None)?;

    Ok(())
}

fn handle_stderr(value: &Arc<std::sync::Mutex<EventFunctions>>, data: &[u8]) -> anyhow::Result<()> {
    value.lock().unwrap().on_output.call(None, &make_args!("stderr", String::from_utf8(data.to_vec()).unwrap()), None)?;

    Ok(())
}

fn handle_progress(value: &Arc<std::sync::Mutex<EventFunctions>>, progress_update: ProgressUpdate) -> anyhow::Result<()> {
    let funcs = value.lock().unwrap();

    match progress_update {
        ProgressUpdate::SetMax(max) => funcs.on_progress.call(None, &make_args!("max", max as i32), None),
        ProgressUpdate::SetProgress(progress) => funcs.on_progress.call(None, &make_args!("progress", progress as i32), None),
        ProgressUpdate::SetLabel(label) => funcs.on_progress.call(None, &make_args!("label", label), None)
    };

    Ok(())
}

impl EventHandler {
    // script handler
    fn run_client(&self, version_id: i32, on_progress: Value, on_output: Value, on_finalization: Value, on_error: Value) -> bool {
        let runner_instance_clone = self.runner_instance.clone();
        let constant_data_clone = self.constant_data.clone();

        let mut runner_instance_content = self.async_runtime.block_on(self.runner_instance.lock());
        let mut join_handle = self.async_runtime.block_on(self.join_handle.lock());

        if runner_instance_content.is_some() {
            return true;
        }

        let (terminator_tx, terminator_rx) = tokio::sync::oneshot::channel();

        let jh = self.async_runtime.spawn(async move {
            let client_version_manifest = &constant_data_clone.client_version_manifest;

            let target = &client_version_manifest.versions[version_id as usize];

            let res = crate::prelauncher::launch(
                client_version_manifest,
                &constant_data_clone.version_manifest,
                target,
                client_version_manifest.loader_versions.get(&target.loader_version).unwrap(),
                LauncherData {
                    on_stdout: handle_stdout,
                    on_stderr: handle_stderr,
                    on_progress: handle_progress,
                    data: Box::new(Arc::new(std::sync::Mutex::new(EventFunctions { on_output, on_progress }))),
                    terminator: terminator_rx
                }
            ).await;

            match res {
                Err(err) => {on_error.call(None, &make_args!(err.to_string()), None).unwrap();},
                _ => {}
            }

            { *runner_instance_clone.lock().await = None; }

            on_finalization.call(None, &make_args!(), None).unwrap();

            ()
        });

        *runner_instance_content = Some(RunnerInstance { terminator: terminator_tx });
        *join_handle = Some(jh);

        return true;
    }

    fn terminate(&self) -> bool {
        let runner_instance = self.runner_instance.clone();
        let join_handle = self.join_handle.clone();

        self.async_runtime.block_on(async move {
            {
                let mut lck = runner_instance.lock().await;

                match lck.take() {
                    Some(inst) => {
                        println!("Sending sigterm");
                        inst.terminator.send(()).unwrap();
                    },
                    _ => {}
                }
            }

            join_handle.lock().await.take().unwrap().await.unwrap();
        });

        return true;
    }

    // script handler
    fn get_versions(&self, mut output: sciter::Value) -> bool {
        let versions = self.constant_data.client_version_manifest.versions
            .iter()
            .enumerate()
            .map(|(idx, x)| {
                let mut val = Value::new();

                val.set_item("idx", idx as i32);
                val.set_item("liquidBounceVersion", &x.name);
                val.set_item("minecraftVersion", &x.mc_version);
                val.set_item("loaderName", &x.loader_version);

                val
            })
            .collect::<Vec<_>>();

        output.set_item("versions", Value::from_iter(versions));

        true
    }

    fn login_mojang(&self, username: String, password: String, on_error: Value, on_response: Value) -> bool {
        self.async_runtime.spawn(async move {
            match AuthService::authenticate(AuthService::MOJANG, username, password).await {
                Ok(acc) => {
                    let mut val = Value::new();

                    val.set_item("username", acc.username);
                    val.set_item("accessToken", acc.access_token);
                    val.set_item("id", acc.id.to_string());
    
                    on_response.call(None, &make_args!(val), None).unwrap()
                },
                Err(err) => {
                    println!("{:?}", err);

                    on_error.call(None, &make_args!(err.to_string()), None).unwrap()
                }
            };

            ()
        });

        true
    }

}

impl sciter::EventHandler for EventHandler {
    fn get_subscription(&mut self) -> Option<sciter::dom::event::EVENT_GROUPS> {
        Some(sciter::dom::event::default_events() | sciter::dom::event::EVENT_GROUPS::HANDLE_METHOD_CALL)
    }

    // route script calls to our handler
    dispatch_script_call! {
		fn run_client(i32, Value, Value, Value, Value);
		fn terminate();
		fn get_versions(Value);
        fn login_mojang(String, String, Value, Value);
	}
}


/// Runs the GUI and returns when the window is closed.
pub(crate) fn gui_main() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("Failed to open runtime");

    let client_version_manifest = rt.block_on(ClientVersionManifest::load_version_manifest()).expect("Failed to download version manifest");

    if client_version_manifest.file_version > SUPPORTED_CLOUD_FILE_VERSION {
        eprintln!("ERROR: Unsupported version manifest");
        return;
    }

    let version_manifest = rt.block_on(VersionManifest::download()).expect("Failed to download version manifest");

    let gui_index = get_gui_index().expect("unable to find gui index");

    let mut frame = sciter::WindowBuilder::main_window()
        .glassy()
        .alpha()
        .fixed()
        .debug()
        .with_size((1000, 600))
        .create();

    frame.event_handler(EventHandler { constant_data: Arc::new(ConstantLauncherData { version_manifest, client_version_manifest }), runner_instance: Arc::new(Mutex::new(None)), join_handle: Arc::new(Default::default()), async_runtime: Runtime::new().unwrap() });

    frame.load_file(&gui_index);
    frame.run_app();
}

fn get_gui_index() -> Result<String> {
    let path = env::current_dir()?;
    let absolut_path = path.absolutize()?;
    let str_path = absolut_path.to_str().expect("no path");

    return Ok(format!("file://{}/gui/public/index.html", str_path));
}