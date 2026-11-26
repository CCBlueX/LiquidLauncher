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

use backon::{ExponentialBuilder, Retryable};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use minecraft_auth::java::JavaAuthManager;
use minecraft_auth::msa::constants::JAVA_TITLE_ID;
use serde_json::json;
use tauri::Window;
use tokio::fs;
use tokio::sync::oneshot;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::{data_directory, failed};
use crate::app::builds::{self, BuildPage};
use crate::app::client_api::{BlogPost, Build, Changelog, Client, JavaVendor, PaginatedResponse};
use crate::app::client_api::LoaderMod;
use crate::app::marketplace;
use crate::app::modrinth::{self, CustomMod};
use crate::app::options::Options;
use crate::auth::{shared, ClientAccount};
use crate::minecraft::game_dir::GameDir;
use crate::minecraft::instance::{Instance, InstanceState};
use crate::minecraft::log::{Level, LogLine};
use crate::{app::gui::{AppState, Runner, RunnerInstance, RunnerInstances}, minecraft::{
    auth::MinecraftAccount,
    launcher::{LauncherData, StartParameter},
    prelauncher,
    progress::ProgressUpdate,
}, HTTP_CLIENT};

/// The build selected to launch, or `None` when the chosen one is gone.
#[tauri::command]
pub(crate) async fn request_build(
    client: Client,
    options: Options,
    app_state: tauri::State<'_, AppState>,
) -> Result<Option<Build>, String> {
    let chosen = options.version_options.build_id != -1;
    let is_gone = |error: &anyhow::Error| {
        chosen
            && error
                .downcast_ref::<reqwest::Error>()
                .and_then(reqwest::Error::status)
                == Some(reqwest::StatusCode::NOT_FOUND)
    };

    let build = (|| async { builds::fetch_selected(&client, &options, &app_state.build).await })
        .retry(ExponentialBuilder::default())
        .when(|error| !is_gone(error))
        .notify(|err, dur| {
            warn!("Failed to request the build. Retrying in {:?}. Error: {}", dur, err);
        })
        .await;

    match build {
        Ok(build) => Ok(Some(build)),
        Err(error) if is_gone(&error) => Ok(None),
        Err(error) => Err(format!("unable to request the build: {:?}", error)),
    }
}

/// A page of the builds to choose from, releases or all of them as the options show them.
#[tauri::command]
pub(crate) async fn request_build_page(
    client: Client,
    options: Options,
    page: u32,
) -> Result<BuildPage, String> {
    builds::page(
        &client,
        page,
        options.launcher_options.show_nightly_builds,
        options.version_options.build_id,
    )
    .await
    .map_err(failed("request builds"))
}

#[tauri::command]
pub(crate) async fn get_java_distributions(client: Client) -> Result<Vec<JavaVendor>, String> {
    client.java_distributions().await.map_err(failed("load Java distributions"))
}

#[tauri::command]
pub(crate) async fn fetch_blog_posts(client: Client, page: u32) -> Result<PaginatedResponse<BlogPost>, String> {
    (|| async { client.blog_posts(page).await })
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to fetch blog posts. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to fetch blog posts: {:?}", e))
}

#[tauri::command]
pub(crate) async fn fetch_changelog(client: Client, build_id: u32) -> Result<Changelog, String> {
    (|| async { client.fetch_changelog(build_id).await })
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to fetch changelog. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to fetch changelog: {:?}", e))
}

#[tauri::command]
pub(crate) async fn request_mods(
    client: Client,
    mc_version: &str,
    subsystem: &str,
) -> Result<Vec<LoaderMod>, String> {
    let mods = (|| async { client.fetch_mods(&mc_version, &subsystem).await })
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to request mods. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to request mods: {:?}", e))?;

    Ok(mods)
}

/// The mods from Modrinth and from files. Without `check` it reads the options and the disk alone
/// and returns at once.
#[tauri::command]
pub(crate) async fn get_custom_mods(
    options: Options,
    branch: &str,
    mc_version: &str,
    subsystem: &str,
    check: bool,
) -> Result<Vec<CustomMod>, String> {
    let branch_options = options
        .version_options
        .options
        .get(branch)
        .cloned()
        .unwrap_or_default();
    let installed = branch_options
        .modrinth_mods
        .get(mc_version)
        .map(Vec::as_slice)
        .unwrap_or_default();

    modrinth::custom_mods(
        &prelauncher::custom_mods_directory(
            &options.start_options.data_directory(),
            branch,
            mc_version,
        ),
        installed,
        &branch_options.custom_mod_states,
        mc_version,
        subsystem,
        check,
    )
    .await
    .map_err(|e| format!("unable to read custom mods: {:?}", e))
}

#[tauri::command]
pub(crate) async fn install_custom_mod(
    options: Options,
    branch: &str,
    mc_version: &str,
    path: PathBuf,
) -> Result<(), String> {
    let mod_cache_path = prelauncher::custom_mods_directory(
        &options.start_options.data_directory(),
        branch,
        mc_version,
    );

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
    options: Options,
    branch: &str,
    mc_version: &str,
    mod_name: &str,
) -> Result<(), String> {
    let mod_cache_path = prelauncher::custom_mods_directory(
        &options.start_options.data_directory(),
        branch,
        mc_version,
    );

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

fn handle_stdout(runner: &Runner, data: &[u8]) -> anyhow::Result<()> {
    let line = String::from_utf8_lossy(data);
    let line = line.trim_end_matches(['\r', '\n']);
    info!("{}", line);
    output(runner, LogLine::parse(line))
}

fn handle_stderr(runner: &Runner, data: &[u8]) -> anyhow::Result<()> {
    let line = String::from_utf8_lossy(data);
    let line = line.trim_end_matches(['\r', '\n']);
    error!("{}", line);
    output(runner, LogLine::parse(line))
}

fn handle_progress(runner: &Runner, progress_update: ProgressUpdate) -> anyhow::Result<()> {
    runner.emit("progress-update", &progress_update)?;

    // Check if progress update is label update
    if let ProgressUpdate::SetLabel(label) = progress_update {
        handle_log(runner, &label)?;
    }
    Ok(())
}

fn handle_log(runner: &Runner, msg: &str) -> anyhow::Result<()> {
    info!("{}", msg);
    let _ = output(runner, LogLine::Launcher { text: msg.to_owned(), level: Level::Info });
    Ok(())
}

fn handle_error(runner: &Runner, msg: &str) -> anyhow::Result<()> {
    error!("{}", msg);
    output(runner, LogLine::Launcher { text: msg.to_owned(), level: Level::Error })
}

fn output(runner: &Runner, line: LogLine) -> anyhow::Result<()> {
    runner.emit("process-output", json!({ "id": runner.id, "line": line }))
}

fn add_runner(
    instances: &RunnerInstances,
    window: Window,
    build: &Build,
    username: &str,
) -> Result<(Runner, oneshot::Receiver<()>), String> {
    static NEXT_ID: AtomicU32 = AtomicU32::new(1);

    let mut runners = instances
        .lock()
        .map_err(|e| format!("unable to lock runner instances: {:?}", e))?;
    let is_starting = |runner: &RunnerInstance| runner.instance.state == InstanceState::Starting;
    if runners.values().any(is_starting) {
        return Err("another instance is still starting".to_string());
    }

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let (terminator, terminated) = oneshot::channel();
    runners.insert(
        id,
        RunnerInstance {
            terminator: Some(terminator),
            instance: Instance {
                id,
                pid: None,
                version: build.mc_version.clone(),
                lb_version: build.lb_version.clone(),
                username: username.to_owned(),
                state: InstanceState::Starting,
            },
        },
    );

    let runner = Runner {
        id,
        // A shared mutex for the window object.
        window: Arc::new(Mutex::new(window)),
        instances: instances.clone(),
    };
    Ok((runner, terminated))
}

#[tauri::command]
pub(crate) async fn run_client(
    client: Client,
    build_id: u32,
    options: Options,
    mods: Vec<LoaderMod>,
    window: Window,
    app_state: tauri::State<'_, AppState>,
) -> Result<u32, String> {
    let data = data_directory(&options).await?;

    let minecraft_account = options
        .start_options
        .minecraft_account
        .ok_or("no account selected")?;
    let (account_name, uuid, token, user_type) = match minecraft_account {
        MinecraftAccount::MsaAccount { state, name, id } => {
            let manager = JavaAuthManager::from_json(HTTP_CLIENT.clone(), &state)
                .map_err(|e| format!("unable to load account: {}", e))?;
            let token = manager
                .minecraft_token()
                .await
                .map_err(|e| format!("unable to refresh account: {}", e))?;
            (name, id.to_string(), token.access_token, "msa".to_string())
        }
        MinecraftAccount::OfflineAccount { name, id, .. } => {
            (name, id.to_string(), "-".to_string(), "legacy".to_string())
        }
    };

    // Random XUID
    let xuid = Uuid::new_v4().to_string();

    info!("Loading launch manifest...");
    let launch_manifest = client.fetch_launch_manifest(build_id).await.map_err(|e| {
        format!(
            "failed to fetch launch manifest of build {}: {:?}",
            build_id, e
        )
    })?;

    let (runner, terminator_rx) = add_runner(
        &app_state.runner_instances,
        window,
        &launch_manifest.build,
        &account_name,
    )?;
    marketplace::game_started();

    let client_account = if options.premium_options.skip_advertisement {
        premium_account(&client, &data)
            .await
            .unwrap_or_else(|e| {
                let message = format!("Failed to authenticate client account: {:?}", e);
                let _ = handle_error(&runner, &message);
                None
            })
    } else {
        None
    };
    let skip_advertisement = client_account.is_some();

    let id = runner.id;

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
        clientid: JAVA_TITLE_ID.to_string(),
        user_type,
        keep_launcher_open: options.launcher_options.keep_launcher_open,
        concurrent_downloads: options.launcher_options.concurrent_downloads,
        client,
        client_account,
        skip_advertisement,
        vanilla_integration: options.start_options.installation,
    };

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
                    on_start: |runner, pid| {
                        runner.update(|instance| {
                            instance.pid = Some(pid);
                            instance.state = InstanceState::Started;
                        })
                    },
                    hide_window: |runner| runner.window.lock().unwrap().hide().unwrap(),
                    data: Box::new(runner.clone()),
                    terminator: terminator_rx,
                };

                if let Err(e) =
                    prelauncher::launch(launch_manifest, parameters, mods, launcher_data).await
                {
                    if !keep_launcher_open {
                        runner.window.lock().unwrap().show().unwrap();
                    }

                    let message = format!("An error occurred:\n\n{:?}", e);
                    runner
                        .emit("client-error", json!({ "id": runner.id }))
                        .unwrap();
                    handle_error(&runner, &message).unwrap();
                };
                marketplace::game_exited(&GameDir::new(data)).await;

                runner.instances.lock().unwrap().remove(&runner.id);
                runner
                    .emit("client-exited", json!({ "id": runner.id }))
                    .unwrap()
            });
    });

    Ok(id)
}

#[tauri::command]
pub(crate) async fn list_instances(
    app_state: tauri::State<'_, AppState>,
) -> Result<Vec<Instance>, String> {
    let instances = app_state
        .runner_instances
        .lock()
        .map_err(|e| format!("unable to lock runner instances: {:?}", e))?
        .values()
        .map(|runner| runner.instance.clone())
        .collect();
    Ok(instances)
}

async fn premium_account(client: &Client, data: &Path) -> anyhow::Result<Option<ClientAccount>> {
    let Some(account) = shared::read(data).await? else {
        return Ok(None);
    };
    let account = shared::renew(data, account).await?;
    let premium = client.fetch_user(&account).await?.premium;
    Ok(premium.then_some(account))
}

#[tauri::command]
pub(crate) async fn terminate(
    id: u32,
    app_state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let terminator = app_state
        .runner_instances
        .lock()
        .map_err(|e| format!("unable to lock runner instances: {:?}", e))?
        .get_mut(&id)
        .and_then(|runner| runner.terminator.take());

    if let Some(terminator) = terminator {
        info!("Stopping instance {id}");
        let _ = terminator.send(());
    }
    Ok(())
}
