use std::{process::exit, sync::Arc};

use env_logger::Env;
use log::{info};
use sysinfo::SystemExt;
use tauri::{Manager, Window};

use crate::{LAUNCHER_DIRECTORY, minecraft::{service::{Account, self}, launcher::{LaunchingParameter, LauncherData}, progress::ProgressUpdate, prelauncher}};

use super::{app_data::LauncherOptions, api::{ApiEndpoints, Build, LoaderMod}};

#[tauri::command]
fn exit_app() {
    // exit app
    exit(0);
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

struct ProgressState {
    window: Window,
}

fn handle_stdout(value: &Arc<std::sync::Mutex<ProgressState>>, data: &[u8]) -> anyhow::Result<()> {
    value.lock().unwrap().window.emit("process-output", String::from_utf8(data.to_vec())?)?;
    Ok(())
}

fn handle_stderr(value: &Arc<std::sync::Mutex<ProgressState>>, data: &[u8]) -> anyhow::Result<()> {
    value.lock().unwrap().window.emit("process-output", String::from_utf8(data.to_vec())?)?;
    Ok(())
}

fn handle_progress(value: &Arc<std::sync::Mutex<ProgressState>>, progress_update: ProgressUpdate) -> anyhow::Result<()> {
    value.lock().unwrap().window.emit("progress-update", progress_update)?;
    Ok(())
}

#[tauri::command]
async fn run_client(build_id: i32, account_data: Account, options: LauncherOptions, mods: Vec<LoaderMod>, window: Window) -> Result<(), String> {
    let (account_name, uuid, token, user_type) = match account_data {
        Account::MsaAccount { auth, .. } => (auth.name, auth.uuid, auth.token, "msa".to_string()),
        Account::MojangAccount { name, token, uuid } => (name, token, uuid, "mojang".to_string()),
        Account::OfflineAccount { name, uuid } => (name, "-".to_string(), uuid, "legacy".to_string())
    };

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

    info!("Loading launch manifest...");
    let launch_manifest = ApiEndpoints::launch_manifest(build_id)
        .await
        .map_err(|e| format!("unable to request launch manifest: {:?}", e))?;

    prelauncher::launch(
            launch_manifest,
            parameters,
            mods,
            LauncherData {
                on_stdout: handle_stdout,
                on_stderr: handle_stderr,
                on_progress: handle_progress,
                data: Box::new(Arc::new(std::sync::Mutex::new(ProgressState {
                    window
                }))),
                terminator: terminator_rx
            }
    ).await
        .map_err(|e| format!("failed to launch client: {:?}", e))?;

    Ok(())
}

/// Runs the GUI and returns when the window is closed.
pub(crate) fn gui_main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("liquidlauncher=debug")).init();

    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
            }
            

            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::apply_acrylic;
                apply_acrylic(&window, Some((18, 18, 18, 125)))
                .expect("Unsupported platform! 'apply_blur' is only supported on Windows");
            }
            

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            exit_app,
            open_url,
            get_options,
            store_options,
            request_branches,
            request_builds,
            request_mods,
            run_client
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
