use std::{sync::{Arc, Mutex}, thread};

use env_logger::Env;
use log::{info, error};
use sysinfo::SystemExt;
use tauri::{Manager, Window};

use crate::{LAUNCHER_DIRECTORY, minecraft::{service::{Account, self}, launcher::{LaunchingParameter, LauncherData}, progress::ProgressUpdate, prelauncher}};

use super::{app_data::LauncherOptions, api::{ApiEndpoints, Build, LoaderMod}};

struct RunnerInstance {
    terminator: tokio::sync::oneshot::Sender<()>,
}

struct AppState {
    runner_instance: Arc<Mutex<Option<RunnerInstance>>>
}

#[tauri::command]
fn open_url(url: &str) -> Result<(), String> {
    open::that(url)
        .map_err(|e| format!("unable to open url: {:?}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_options() -> Result<LauncherOptions, String> {
    let config_dir = LAUNCHER_DIRECTORY.config_dir();
    let options = LauncherOptions::load(config_dir).await.unwrap_or_default(); // default to basic options if unable to load
    
    Ok(options)
}

#[tauri::command]
async fn store_options(options: LauncherOptions) -> Result<(), String> {
    let config_dir = LAUNCHER_DIRECTORY.config_dir();
    options.store(config_dir)
        .await
        .map_err(|e| format!("unable to store config data: {:?}", e))?;

    Ok(())
}

#[tauri::command]
async fn request_branches() -> Result<Vec<String>, String> {
    let branches = ApiEndpoints::branches()
        .await
        .map_err(|e| format!("unable to request branches: {:?}", e))?;
    
    Ok(branches)
}

#[tauri::command]
async fn request_builds(branch: &str) -> Result<Vec<Build>, String> {
    let builds = ApiEndpoints::builds_by_branch(branch)
        .await
        .map_err(|e| format!("unable to request builds: {:?}", e))?;
    
    Ok(builds)
}

#[tauri::command]
async fn request_mods(mc_version: &str, subsystem: &str) -> Result<Vec<LoaderMod>, String> {
    let mods = ApiEndpoints::mods(&mc_version, &subsystem)
        .await
        .map_err(|e| format!("unable to request mods: {:?}", e))?;

    Ok(mods)
}

#[tauri::command]
async fn login_offline(username: &str) -> Result<Account, String> {
    let account = service::auth_offline(username.to_string())
        .await;

    Ok(account)
}

#[tauri::command]
fn login_microsoft(window: tauri::Window) -> Result<(), String> {
    // todo: rewrite library async
    thread::spawn(move || {
        let account = service::auth_msa(|code| {
            info!("received code: {}", code);

            let _ = window.emit("microsoft_code", code);
        }).unwrap(); // unwrap is fine cuz own thread

        let _ = window.emit("microsoft_successful", account);
    });

  Ok(())
}

fn handle_stdout(window: &Arc<std::sync::Mutex<Window>>, data: &[u8]) -> anyhow::Result<()> {
    window.lock().unwrap().emit("process-output", String::from_utf8(data.to_vec())?)?;
    Ok(())
}

fn handle_stderr(window: &Arc<std::sync::Mutex<Window>>, data: &[u8]) -> anyhow::Result<()> {
    window.lock().unwrap().emit("process-output", String::from_utf8(data.to_vec())?)?;
    Ok(())
}

fn handle_progress(window: &Arc<std::sync::Mutex<Window>>, progress_update: ProgressUpdate) -> anyhow::Result<()> {
    window.lock().unwrap().emit("progress-update", progress_update)?;
    Ok(())
}

#[tauri::command]
async fn run_client(build_id: i32, account_data: Account, options: LauncherOptions, mods: Vec<LoaderMod>, window: Window, app_state: tauri::State<'_, AppState>) -> Result<(), String> {
    let window_mutex = Arc::new(std::sync::Mutex::new(window));

    let (account_name, uuid, token, user_type) = match account_data {
        Account::MsaAccount { auth, .. } => (auth.name, auth.uuid, auth.token, "msa".to_string()),
        Account::MojangAccount { name, token, uuid } => (name, token, uuid, "mojang".to_string()),
        Account::OfflineAccount { name, uuid } => (name, "-".to_string(), uuid, "legacy".to_string())
    };

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

    let runner_instance = &app_state.runner_instance;

    if runner_instance.lock().map_err(|e| format!("unable to lock runner instance: {:?}", e))?.is_some() {
        return Err("client is already running".to_string());
    }
    
    info!("Loading launch manifest...");
    let launch_manifest = ApiEndpoints::launch_manifest(build_id)
        .await
        .map_err(|e| format!("unable to request launch manifest: {:?}", e))?;

    let (terminator_tx, terminator_rx) = tokio::sync::oneshot::channel();

    *runner_instance.lock().map_err(|e| format!("unable to lock runner instance: {:?}", e))?
        = Some(RunnerInstance { terminator: terminator_tx });

    prelauncher::launch(
            launch_manifest,
            parameters,
            mods,
            LauncherData {
                on_stdout: handle_stdout,
                on_stderr: handle_stderr,
                on_progress: handle_progress,
                data: Box::new(window_mutex.clone()),
                terminator: terminator_rx
            },
            window_mutex.clone()
    ).await
        .map_err(|e| format!("Failed to launch client: {:?}", e))?;

    *runner_instance.lock().map_err(|e| format!("unable to lock runner instance: {:?}", e))?
        = None;

    Ok(())
}

#[tauri::command]
async fn terminate(app_state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut lck = app_state.runner_instance.lock()
        .map_err(|e| format!("unable to lock runner instance: {:?}", e))?;

    if let Some(inst) = lck.take() {
        println!("Sending sigterm");
        inst.terminator.send(()).unwrap();
    }
    Ok(())
}

/// Runs the GUI and returns when the window is closed.
pub fn gui_main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("liquidlauncher=debug")).init();

    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                if let Err(e) = apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None) {
                    error!("Failed to apply vibrancy: {:?}", e);
                }
            }
            
            // Applies blur to the window and make corners rounded
            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::{apply_acrylic, apply_rounded_corners, apply_blur};

                if let Err(e) = apply_acrylic(&window, None) {
                    error!("Failed to apply acrylic vibrancy: {:?}", e);

                    if let Err(e) = apply_blur(&window) {
                        error!("Failed to apply blur vibrancy: {:?}", e);
                    }
                }

                if let Err(e) = apply_rounded_corners(&window) {
                    error!("Failed to apply rounded corners: {:?}", e);
                    
                    // todo: fallback to HTML corners
                }
            }

            Ok(())
        })
        .manage(AppState { 
            runner_instance: Arc::new(Mutex::new(None))
        })
        .invoke_handler(tauri::generate_handler![
            open_url,
            get_options,
            store_options,
            request_branches,
            request_builds,
            request_mods,
            run_client,
            login_offline,
            login_microsoft,
            terminate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
