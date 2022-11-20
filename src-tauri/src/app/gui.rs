use std::process::exit;

use env_logger::Env;
use tauri::Manager;

use crate::LAUNCHER_DIRECTORY;

use super::{app_data::LauncherOptions, api::{ApiEndpoints, Build}};

#[tauri::command]
fn exit_app() {
    // exit app
    exit(0);
}

#[tauri::command]
fn open_url(url: &str) -> Result<(), String> {
    open::that(url)
        .map_err(|e| format!("unable to store config data: {:?}", e))?;
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
    let branches = ApiEndpoints::builds_by_branch(branch)
        .await
        .map_err(|e| format!("unable to request builds: {:?}", e))?;
    
    Ok(branches)
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
                use window_vibrancy::{apply_acrylic};
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
            request_builds
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
