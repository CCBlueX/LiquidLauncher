/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2024 CCBlueX
 *
 * LiquidLauncher is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * LiquidLauncher is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with LiquidLauncher. If not, see <https://www.gnu.org/licenses/>.
 */
 
use std::{sync::{Arc, Mutex}, thread, path::PathBuf};

use tokio::fs;
use tracing::{error, info, debug};
use tauri::{Manager, Window};
use uuid::Uuid;

use crate::{LAUNCHER_DIRECTORY, minecraft::{launcher::{LauncherData, LaunchingParameter}, prelauncher, progress::ProgressUpdate, auth::{MinecraftAccount, self}}, HTTP_CLIENT, LAUNCHER_VERSION};
use crate::app::api::{Branches, Changelog, ContentDelivery, News};
use crate::utils::percentage_of_total_memory;

use super::{api::{ApiEndpoints, Build, LoaderMod, ModSource}, app_data::LauncherOptions};

struct RunnerInstance {
    terminator: tokio::sync::oneshot::Sender<()>,
}

struct AppState {
    runner_instance: Arc<Mutex<Option<RunnerInstance>>>
}

#[tauri::command]
async fn get_launcher_version() -> Result<String, String> {
    Ok(LAUNCHER_VERSION.to_string())
}

#[tauri::command]
async fn check_online_status() -> Result<(), String> {
    HTTP_CLIENT.get("https://api.liquidbounce.net/")
        .send().await
        .map_err(|e| format!("unable to connect to api.liquidbounce.net: {:}", e))?
        .error_for_status()
        .map_err(|e| format!("api.liquidbounce.net returned an error: {:}", e))?;
    Ok(())
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
async fn request_branches() -> Result<Branches, String> {
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
async fn request_mods(branch: &str, mc_version: &str, subsystem: &str) -> Result<Vec<LoaderMod>, String> {
    let mods = ApiEndpoints::mods(&mc_version, &subsystem)
        .await
        .map_err(|e| format!("unable to request mods: {:?}", e))?;

    Ok(mods)
}

#[tauri::command]
async fn login_offline(username: &str) -> Result<MinecraftAccount, String> {
    let account = MinecraftAccount::auth_offline(username.to_string())
        .await;

    Ok(account)
}

#[tauri::command]
async fn login_microsoft(window: tauri::Window) -> Result<MinecraftAccount, String> {
    let account = MinecraftAccount::auth_msa(|code| {
        debug!("received code: {}", code);

        let _ = window.emit("microsoft_code", code);
    }).await.map_err(|e| format!("unable to ms auth: {:?}", e))?;

  Ok(account)
}

#[tauri::command]
async fn get_custom_mods(branch: &str, mc_version: &str) -> Result<Vec<LoaderMod>, String> {
    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_cache_path = data.join("custom_mods").join(format!("{}-{}", branch, mc_version));

    if !mod_cache_path.exists() {
        return Ok(vec![]);
    }

    let mut mods = vec![];
    let mut mods_read = fs::read_dir(&mod_cache_path).await
        .map_err(|e| format!("unable to read custom mods: {:?}", e))?;

    while let Some(entry) = mods_read.next_entry().await.map_err(|e| format!("unable to read custom mods: {:?}", e))? {
        let file_type = entry.file_type().await
            .map_err(|e| format!("unable to read custom mods: {:?}", e))?;
        let file_name = entry.file_name().to_str().unwrap().to_string();

        if file_type.is_file() && file_name.ends_with(".jar") {
            // todo: pull name from JAR manifest
            let file_name_without_extension = file_name.replace(".jar", "");
            
            mods.push(LoaderMod { required: false, enabled: true, name: file_name_without_extension, source: ModSource::Local { file_name } });
        }
    }

    Ok(mods)
}

#[tauri::command]
async fn install_custom_mod(branch: &str, mc_version: &str, path: PathBuf) -> Result<(), String> {
    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_cache_path = data.join("custom_mods").join(format!("{}-{}", branch, mc_version));

    if !mod_cache_path.exists() {
        fs::create_dir_all(&mod_cache_path).await.unwrap();
    }

    if let Some(file_name) = path.file_name() {
        let dest_path = mod_cache_path.join(file_name.to_str().unwrap());

        fs::copy(path, dest_path).await
            .map_err(|e| format!("unable to copy custom mod: {:?}", e))?;
        return Ok(());
    }
    
    return Err("unable to copy custom mod: invalid path".to_string());
}

#[tauri::command]
async fn delete_custom_mod(branch: &str, mc_version: &str, mod_name: &str) -> Result<(), String> {
    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_cache_path = data.join("custom_mods").join(format!("{}-{}", branch, mc_version));

    if !mod_cache_path.exists() {
        return Ok(());
    }

    let mod_path = mod_cache_path.join(mod_name);

    if mod_path.exists() {
        fs::remove_file(mod_path).await
            .map_err(|e| format!("unable to delete custom mod: {:?}", e))?;
    }

    Ok(())
}

fn handle_stdout(window: &Arc<std::sync::Mutex<Window>>, data: &[u8]) -> anyhow::Result<()> {
    let data = String::from_utf8(data.to_vec())?;
    if data.is_empty() {
        return Ok(()); // ignore empty lines
    }

    info!("{}", data);
    window.lock().unwrap().emit("process-output", data)?;
    Ok(())
}

fn handle_stderr(window: &Arc<std::sync::Mutex<Window>>, data: &[u8]) -> anyhow::Result<()> {
    let data = String::from_utf8(data.to_vec())?;
    if data.is_empty() {
        return Ok(()); // ignore empty lines
    }

    error!("{}", data);
    window.lock().unwrap().emit("process-output", data)?;
    Ok(())
}

fn handle_progress(window: &Arc<std::sync::Mutex<Window>>, progress_update: ProgressUpdate) -> anyhow::Result<()> {
    window.lock().unwrap().emit("progress-update", progress_update)?;
    Ok(())
}

#[tauri::command]
async fn run_client(build_id: u32, account_data: MinecraftAccount, options: LauncherOptions, mods: Vec<LoaderMod>, window: Window, app_state: tauri::State<'_, AppState>) -> Result<(), String> {
    let window_mutex = Arc::new(std::sync::Mutex::new(window));

    let (account_name, uuid, token, user_type) = match account_data {
        MinecraftAccount::MsaAccount { name, uuid, token, .. } => (name, uuid, token, "msa".to_string()),
        MinecraftAccount::OfflineAccount { name, uuid } => (name, uuid, "-".to_string(), "legacy".to_string())
    };

    // Random XUID
    let xuid = Uuid::new_v4().to_string();

    let parameters = LaunchingParameter {
        memory: percentage_of_total_memory(options.memory_percentage),
        custom_data_path: if !options.custom_data_path.is_empty() { Some(options.custom_data_path) } else { None },
        custom_java_path: if !options.custom_java_path.is_empty() { Some(options.custom_java_path) } else { None },
        auth_player_name: account_name,
        auth_uuid: uuid,
        auth_access_token: token,
        auth_xuid: xuid,
        clientid: auth::AZURE_CLIENT_ID.to_string(),
        user_type,
        keep_launcher_open: options.keep_launcher_open,
        concurrent_downloads: options.concurrent_downloads,
    };

    let runner_instance = &app_state.runner_instance;

    if runner_instance.lock().map_err(|e| format!("unable to lock runner instance: {:?}", e))?.is_some() {
        return Err("client is already running".to_string());
    }
    
    info!("Loading launch manifest...");
    let launch_manifest = ApiEndpoints::launch_manifest(build_id)
        .await
        .map_err(|e| format!("failed to fetch launch manifest of build {}: {:?}", build_id, e))?;

    let (terminator_tx, terminator_rx) = tokio::sync::oneshot::channel();

    *runner_instance.lock().map_err(|e| format!("unable to lock runner instance: {:?}", e))?
        = Some(RunnerInstance { terminator: terminator_tx });

    let copy_of_runner_instance = runner_instance.clone();

    thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let keep_launcher_open = parameters.keep_launcher_open;

                if let Err(e) = prelauncher::launch(
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
                ).await {
                    if !keep_launcher_open {
                        window_mutex.lock().unwrap().show().unwrap();
                    }

                    let message = format!("An error with the client occourd:\n{:?}", e);
                    window_mutex.lock().unwrap().emit("client-error", format!("{}\n\nIf this error persists, upload your log with the button below and report it to GitHub.", message)).unwrap();
                    handle_stderr(&window_mutex, message.as_bytes()).unwrap();
                };

                *copy_of_runner_instance.lock().map_err(|e| format!("unable to lock runner instance: {:?}", e)).unwrap()
                    = None;
                window_mutex.lock().unwrap().emit("client-exited", ()).unwrap()
            });
    });



    Ok(())
}

#[tauri::command]
async fn terminate(app_state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut lck = app_state.runner_instance.lock()
        .map_err(|e| format!("unable to lock runner instance: {:?}", e))?;

    if let Some(inst) = lck.take() {
        info!("Sending sigterm");
        inst.terminator.send(()).unwrap();
    }
    Ok(())
}

#[tauri::command]
async fn refresh(account_data: MinecraftAccount) -> Result<MinecraftAccount, String> {
    let account = account_data.refresh().await
        .map_err(|e| format!("unable to refresh: {:?}", e))?;
    Ok(account)
}

#[tauri::command]
async fn logout(account_data: MinecraftAccount) -> Result<(), String> {
    account_data.logout().await.map_err(|e| format!("unable to logout: {:?}", e))
}

#[tauri::command]
async fn mem_percentage(memory_percentage: i32) -> i64 {
    percentage_of_total_memory(memory_percentage)
}

#[tauri::command]
async fn fetch_news() -> Result<Vec<News>, String> {
    ContentDelivery::news()
        .await
        .map_err(|e| format!("unable to fetch news: {:?}", e))
}

#[tauri::command]
async fn fetch_changelog(build_id: u32) -> Result<Changelog, String> {
    ApiEndpoints::changelog(build_id)
        .await
        .map_err(|e| format!("unable to fetch changelog: {:?}", e))
}

#[tauri::command]
async fn default_data_folder_path() -> Result<String, String> {
    let data_directory = LAUNCHER_DIRECTORY.data_dir().to_str();

    match data_directory {
        None => Err("unable to get data folder path".to_string()),
        Some(path) => Ok(path.to_string())
    }
}

#[tauri::command]
async fn clear_data(options: LauncherOptions) -> Result<(), String> {
    let data_directory = if !options.custom_data_path.is_empty() {
        Some(options.custom_data_path)
    } else {
        None
    }.map(|x| x.into()).unwrap_or_else(|| LAUNCHER_DIRECTORY.data_dir().to_path_buf());

    ["assets", "gameDir", "libraries", "mod_cache", "natives", "runtimes", "versions"]
        .iter()
        .map(|dir| data_directory.join(dir))
        .filter(|dir| dir.exists())
        .map(std::fs::remove_dir_all)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("unable to clear data: {:?}", e))?;
    Ok(())
}

/// Runs the GUI and returns when the window is closed.
pub fn gui_main() {
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
                use window_vibrancy::{apply_acrylic, apply_blur, apply_rounded_corners};

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
            check_online_status,
            get_options,
            store_options,
            request_branches,
            request_builds,
            request_mods,
            run_client,
            login_offline,
            login_microsoft,
            logout,
            refresh,
            fetch_news,
            fetch_changelog,
            clear_data,
            mem_percentage,
            default_data_folder_path,
            terminate,
            get_launcher_version,
            get_custom_mods,
            install_custom_mod,
            delete_custom_mod
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
