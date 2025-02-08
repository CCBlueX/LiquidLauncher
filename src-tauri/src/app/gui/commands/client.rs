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

use anyhow::anyhow;
use backon::{ExponentialBuilder, Retryable};
use std::path::PathBuf;
use std::{
    sync::{Arc, Mutex},
    thread,
};
use tauri::{Emitter, Window};
use tokio::fs;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::app::client_api::{LoaderMod, ModSource};
use crate::app::options::Options;
use crate::{app::client_api::{ApiEndpoints, Branches, Build, Changelog, ContentDelivery, News}, app::gui::{AppState, RunnerInstance, ShareableWindow}, minecraft::{
    auth::{self, MinecraftAccount},
    launcher::{LauncherData, StartParameter},
    prelauncher,
    progress::ProgressUpdate,
}, LAUNCHER_DIRECTORY};

#[tauri::command]
pub(crate) async fn request_branches() -> Result<Branches, String> {
    let branches = ApiEndpoints::branches
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to request branches. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to request branches: {:?}", e))?;

    Ok(branches)
}

#[tauri::command]
pub(crate) async fn request_builds(branch: &str, release: bool) -> Result<Vec<Build>, String> {
    let builds = (|| async { ApiEndpoints::builds_by_branch(branch, release).await })
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to request builds. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to request builds: {:?}", e))?;

    Ok(builds)
}

#[tauri::command]
pub(crate) async fn fetch_news() -> Result<Vec<News>, String> {
    ContentDelivery::news
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to fetch news. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to fetch news: {:?}", e))
}

#[tauri::command]
pub(crate) async fn fetch_changelog(build_id: u32) -> Result<Changelog, String> {
    (|| async { ApiEndpoints::changelog(build_id).await })
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to fetch changelog. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to fetch changelog: {:?}", e))
}

#[tauri::command]
pub(crate) async fn request_mods(
    mc_version: &str,
    subsystem: &str,
) -> Result<Vec<LoaderMod>, String> {
    let mods = (|| async { ApiEndpoints::mods(&mc_version, &subsystem).await })
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to request mods. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to request mods: {:?}", e))?;

    Ok(mods)
}

#[tauri::command]
pub(crate) async fn get_custom_mods(
    branch: &str,
    mc_version: &str,
) -> Result<Vec<LoaderMod>, String> {
    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_cache_path = data
        .join("custom_mods")
        .join(format!("{}-{}", branch, mc_version));

    if !mod_cache_path.exists() {
        return Ok(vec![]);
    }

    let mut mods = vec![];
    let mut mods_read = fs::read_dir(&mod_cache_path)
        .await
        .map_err(|e| format!("unable to read custom mods: {:?}", e))?;

    while let Some(entry) = mods_read
        .next_entry()
        .await
        .map_err(|e| format!("unable to read custom mods: {:?}", e))?
    {
        let file_type = entry
            .file_type()
            .await
            .map_err(|e| format!("unable to read custom mods: {:?}", e))?;
        let file_name = entry.file_name().to_str().unwrap().to_string();

        if file_type.is_file() && file_name.ends_with(".jar") {
            // todo: pull name from JAR manifest
            let file_name_without_extension = file_name.replace(".jar", "");

            mods.push(LoaderMod {
                required: false,
                enabled: true,
                name: file_name_without_extension,
                source: ModSource::Local { file_name },
            });
        }
    }

    Ok(mods)
}

#[tauri::command]
pub(crate) async fn install_custom_mod(
    branch: &str,
    mc_version: &str,
    path: PathBuf,
) -> Result<(), String> {
    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_cache_path = data
        .join("custom_mods")
        .join(format!("{}-{}", branch, mc_version));

    if !mod_cache_path.exists() {
        fs::create_dir_all(&mod_cache_path).await.unwrap();
    }

    if let Some(file_name) = path.file_name() {
        let dest_path = mod_cache_path.join(file_name.to_str().unwrap());

        fs::copy(path, dest_path)
            .await
            .map_err(|e| format!("unable to copy custom mod: {:?}", e))?;
        return Ok(());
    }

    Err("unable to copy custom mod: invalid path".to_string())
}

#[tauri::command]
pub(crate) async fn delete_custom_mod(
    branch: &str,
    mc_version: &str,
    mod_name: &str,
) -> Result<(), String> {
    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_cache_path = data
        .join("custom_mods")
        .join(format!("{}-{}", branch, mc_version));

    if !mod_cache_path.exists() {
        return Ok(());
    }

    let mod_path = mod_cache_path.join(mod_name);

    if mod_path.exists() {
        fs::remove_file(mod_path)
            .await
            .map_err(|e| format!("unable to delete custom mod: {:?}", e))?;
    }

    Ok(())
}

fn handle_stdout(window: &ShareableWindow, data: &[u8]) -> anyhow::Result<()> {
    let data = String::from_utf8(data.to_vec())?;
    if data.is_empty() {
        return Ok(()); // ignore empty lines
    }

    info!("{}", data.strip_suffix("\n").unwrap_or(&data));
    window
        .lock()
        .map_err(|_| anyhow!("Window lock is poisoned"))?
        .emit("process-output", data)?;
    Ok(())
}

fn handle_stderr(window: &ShareableWindow, data: &[u8]) -> anyhow::Result<()> {
    let data = String::from_utf8(data.to_vec())?;
    if data.is_empty() {
        return Ok(()); // ignore empty lines
    }

    error!("{}", data.strip_suffix("\n").unwrap_or(&data));
    window
        .lock()
        .map_err(|_| anyhow!("Window lock is poisoned"))?
        .emit("process-output", data)?;
    Ok(())
}

fn handle_progress(
    window: &ShareableWindow,
    progress_update: ProgressUpdate,
) -> anyhow::Result<()> {
    window
        .lock()
        .map_err(|_| anyhow!("Window lock is poisoned"))?
        .emit("progress-update", &progress_update)?;

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
pub(crate) async fn run_client(
    build_id: u32,
    options: Options,
    mods: Vec<LoaderMod>,
    window: Window,
    app_state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // A shared mutex for the window object.
    let shareable_window: ShareableWindow = Arc::new(Mutex::new(window));

    let minecraft_account = options
        .start_options
        .minecraft_account
        .ok_or("no account selected")?;
    let (account_name, uuid, token, user_type) = match minecraft_account {
        MinecraftAccount::MsaAccount {
            msa: _,
            xbl: _,
            mca,
            profile,
            ..
        } => (
            profile.name,
            profile.id.to_string(),
            mca.data.access_token,
            "msa".to_string(),
        ),
        MinecraftAccount::LegacyMsaAccount {
            name, uuid, token, ..
        } => (name, uuid.to_string(), token, "msa".to_string()),
        MinecraftAccount::OfflineAccount { name, id, .. } => {
            (name, id.to_string(), "-".to_string(), "legacy".to_string())
        }
    };

    let client_account = options.premium_options.account;
    let skip_advertisement = options.premium_options.skip_advertisement
        && client_account
        .as_ref()
        .is_some_and(|x| x.get_user_information().is_some_and(|u| u.premium));

    // Random XUID
    let xuid = Uuid::new_v4().to_string();

    let parameters = StartParameter {
        java_distribution: options.start_options.java_distribution,
        jvm_args: options.start_options.jvm_args.unwrap_or_else(|| vec![]),
        memory: options.start_options.memory,
        custom_data_path: if !options.start_options.custom_data_path.is_empty() {
            Some(options.start_options.custom_data_path)
        } else {
            None
        },
        auth_player_name: account_name,
        auth_uuid: uuid,
        auth_access_token: token,
        auth_xuid: xuid,
        clientid: auth::AZURE_CLIENT_ID.to_string(),
        user_type,
        keep_launcher_open: options.launcher_options.keep_launcher_open,
        concurrent_downloads: options.launcher_options.concurrent_downloads,
        client_account,
        skip_advertisement,
    };

    let runner_instance = &app_state.runner_instance;

    if runner_instance
        .lock()
        .map_err(|e| format!("unable to lock runner instance: {:?}", e))?
        .is_some()
    {
        return Err("client is already running".to_string());
    }

    info!("Loading launch manifest...");
    let launch_manifest = ApiEndpoints::launch_manifest(build_id).await.map_err(|e| {
        format!(
            "failed to fetch launch manifest of build {}: {:?}",
            build_id, e
        )
    })?;

    let (terminator_tx, terminator_rx) = tokio::sync::oneshot::channel();

    *runner_instance
        .lock()
        .map_err(|e| format!("unable to lock runner instance: {:?}", e))? = Some(RunnerInstance {
        terminator: terminator_tx,
    });

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
                    terminator: terminator_rx,
                };

                if let Err(e) =
                    prelauncher::launch(launch_manifest, parameters, mods, launcher_data).await
                {
                    if !keep_launcher_open {
                        shareable_window.lock().unwrap().show().unwrap();
                    }

                    let message = format!("An error occured:\n\n{:?}", e);
                    shareable_window
                        .lock()
                        .unwrap()
                        .emit("client-error", ())
                        .unwrap();
                    handle_stderr(&shareable_window, message.as_bytes()).unwrap();
                };

                *copy_of_runner_instance
                    .lock()
                    .map_err(|e| format!("unable to lock runner instance: {:?}", e))
                    .unwrap() = None;
                shareable_window
                    .lock()
                    .unwrap()
                    .emit("client-exited", ())
                    .unwrap()
            });
    });

    Ok(())
}

#[tauri::command]
pub(crate) async fn terminate(app_state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut lck = app_state
        .runner_instance
        .lock()
        .map_err(|e| format!("unable to lock runner instance: {:?}", e))?;

    if let Some(inst) = lck.take() {
        info!("Sending sigterm");
        inst.terminator.send(()).unwrap();
    }
    Ok(())
}
