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

use std::{path::PathBuf, sync::{Arc, Mutex}, thread};

use anyhow::anyhow;
use tauri::{Manager, Window};
use tokio::fs;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::app::api::{Branches, Changelog, ContentDelivery, News};
use crate::utils::percentage_of_total_memory;
use crate::{auth::{ClientAccount, ClientAccountAuthenticator}, minecraft::{auth::{self, MinecraftAccount}, launcher::{LauncherData, LaunchingParameter}, prelauncher, progress::ProgressUpdate}, HTTP_CLIENT, LAUNCHER_DIRECTORY, LAUNCHER_VERSION};

use super::{api::{ApiEndpoints, Build, LoaderMod, ModSource}, app_data::LauncherOptions};

pub type ShareableWindow = Arc<Mutex<Window>>;

struct RunnerInstance {
    terminator: tokio::sync::oneshot::Sender<()>,
}

struct AppState {
    runner_instance: Arc<Mutex<Option<RunnerInstance>>>
}

const ERROR_MSG: &str = "Try restarting the LiquidLauncher with administrator rights.\nIf this error persists, upload your log with the button below and report it to GitHub.";

#[tauri::command]
async fn get_launcher_version() -> Result<String, String> {
    Ok(LAUNCHER_VERSION.to_string())
}

#[tauri::command]
async fn check_health() -> Result<(), String> {
    // Check hosts
    #[cfg(windows)]
    {
        use crate::utils::check_hosts_file;

        info!("Checking hosts file...");
        check_hosts_file().await
            .map_err(|e| format!("{}", e))?;
    }

    info!("Checking online status");
    HTTP_CLIENT.get("https://api.liquidbounce.net/")
        .send().await
        .map_err(|e| format!("unable to connect to api.liquidbounce.net: {:}", e))?
        .error_for_status()
        .map_err(|e| format!("api.liquidbounce.net returned an error: {:}", e))?;
    info!("Online status check successful");
    Ok(())
}

#[tauri::command]
async fn get_options() -> Result<LauncherOptions, String> {
    info!("Loading options...");
    let config_dir = LAUNCHER_DIRECTORY.config_dir();
    let options = LauncherOptions::load(config_dir)
        .await
        .unwrap_or_default(); // default to basic options if unable to load
    info!("Done!");
    
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
async fn request_mods(mc_version: &str, subsystem: &str) -> Result<Vec<LoaderMod>, String> {
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
    let account = MinecraftAccount::auth_msa(|uri, code| {
        debug!("enter code {} on {} to sign-in", code, uri);

        let _ = window.emit("microsoft_code", code);
    }).await.map_err(|e| format!("{}", e))?;

  Ok(account)
}

#[tauri::command]
async fn client_account_authenticate(window: tauri::Window) -> Result<ClientAccount, String> {
    let mut account = ClientAccountAuthenticator::start_auth(|uri| {
        // Open the browser with the auth URL
        let _ = window.emit("auth_url", uri);
    }).await.map_err(|e| format!("{}", e))?;

    // Fetch user information
    account.update_info().await
        .map_err(|e| format!("unable to fetch user information: {:?}", e))?;

  Ok(account)
}

#[tauri::command]
async fn client_account_update(account: ClientAccount) -> Result<ClientAccount, String> {
    let mut account = account.renew().await
        .map_err(|e| format!("unable to update access token: {:?}", e))?;

    // Fetch user information
    account.update_info().await
        .map_err(|e| format!("unable to fetch user information: {:?}", e))?;
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

fn handle_stdout(window: &ShareableWindow, data: &[u8]) -> anyhow::Result<()> {
    let data = String::from_utf8(data.to_vec())?;
    if data.is_empty() {
        return Ok(()); // ignore empty lines
    }

    info!("{}", data);
    window.lock().map_err(|_| anyhow!("Window lock is poisoned"))?.emit("process-output", data)?;
    Ok(())
}

fn handle_stderr(window: &ShareableWindow, data: &[u8]) -> anyhow::Result<()> {
    let data = String::from_utf8(data.to_vec())?;
    if data.is_empty() {
        return Ok(()); // ignore empty lines
    }

    error!("{}", data);
    window.lock().map_err(|_| anyhow!("Window lock is poisoned"))?.emit("process-output", data)?;
    Ok(())
}

fn handle_progress(window: &ShareableWindow, progress_update: ProgressUpdate) -> anyhow::Result<()> {
    window.lock().map_err(|_| anyhow!("Window lock is poisoned"))?.emit("progress-update", &progress_update)?;

    // Check if progress update is label update
    if let ProgressUpdate::SetLabel(label) = progress_update {
        handle_log(window, &label)?;
    }
    Ok(())
}

fn handle_log(window: &ShareableWindow, msg: &str) -> anyhow::Result<()> {
    info!("{}", msg);
    
    if let Ok(k) = window.lock() {
        let _ = k.emit("process-output", msg);
    }
    Ok(())
}

#[tauri::command]
async fn run_client(
    build_id: u32,
    options: LauncherOptions,
    mods: Vec<LoaderMod>,
    window: Window,
    app_state: tauri::State<'_, AppState>
) -> Result<(), String> {
    // A shared mutex for the window object.
    let shareable_window: ShareableWindow = Arc::new(Mutex::new(window));

    let minecraft_account = options.current_account.ok_or("no account selected")?;
    let (account_name, uuid, token, user_type) = match minecraft_account {
        MinecraftAccount::MsaAccount { msa: _, xbl: _, mca, profile, .. } => (profile.name, profile.id.to_string(), mca.data.access_token, "msa".to_string()),
        MinecraftAccount::LegacyMsaAccount { name, uuid, token, .. } => (name, uuid.to_string(), token, "msa".to_string()),
        MinecraftAccount::OfflineAccount { name, id, .. } => (name, id.to_string(), "-".to_string(), "legacy".to_string())
    };
    
    let client_account = options.client_account;
    let skip_advertisement = options.skip_advertisement && client_account.as_ref().is_some_and(|x| 
        x.get_user_information().is_some_and(|u| u.premium)
    );

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
        client_account,
        skip_advertisement: skip_advertisement
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

                let launcher_data = LauncherData {
                    on_stdout: handle_stdout,
                    on_stderr: handle_stderr,
                    on_progress: handle_progress,
                    on_log: handle_log,
                    hide_window: |w| w.lock().unwrap().hide().unwrap(),
                    data: Box::new(shareable_window.clone()),
                    terminator: terminator_rx
                };

                if let Err(e) = prelauncher::launch(
                    launch_manifest,
                    parameters,
                    mods,
                    launcher_data
                ).await {
                    if !keep_launcher_open {
                        shareable_window.lock().unwrap().show().unwrap();
                    }

                    let message = format!("An error occured:\n\n{:?}", e);
                    shareable_window.lock().unwrap().emit("client-error", format!("{}\n\n{}", message, ERROR_MSG)).unwrap();
                    handle_stderr(&shareable_window, message.as_bytes()).unwrap();
                };

                *copy_of_runner_instance.lock().map_err(|e| format!("unable to lock runner instance: {:?}", e)).unwrap()
                    = None;
                shareable_window.lock().unwrap().emit("client-exited", ()).unwrap()
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
    info!("Refreshing account...");
    let account = account_data.refresh().await
        .map_err(|e| format!("unable to refresh: {:?}", e))?;
    info!("Account was refreshed - username {}", account.get_username());
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { 
            runner_instance: Arc::new(Mutex::new(None))
        })
        .invoke_handler(tauri::generate_handler![
            check_health,
            get_options,
            store_options,
            request_branches,
            request_builds,
            request_mods,
            run_client,
            login_offline,
            login_microsoft,
            client_account_authenticate,
            client_account_update,
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
