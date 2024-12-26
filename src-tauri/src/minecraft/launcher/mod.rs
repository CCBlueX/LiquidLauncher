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

use std::collections::HashSet;
use std::path::Path;

use std::process::exit;

use anyhow::{bail, Context, Result};

use path_absolutize::Absolutize;
use tracing::*;

use crate::app::client_api::LaunchManifest;
use crate::auth::ClientAccount;
use crate::error::LauncherError;
use crate::minecraft::java::{DistributionSelection, JavaRuntime};
use crate::minecraft::progress::{ProgressReceiver, ProgressUpdate};
use crate::{join_and_mkdir, join_and_mkdir_vec};
use crate::{
    utils::{OS, OS_VERSION},
    LAUNCHER_VERSION,
};

use self::assets::setup_assets;
use self::client_jar::setup_client_jar;
use self::jre::load_jre;
use self::libraries::setup_libraries;

use super::version::VersionProfile;

mod assets;
mod client_jar;
mod jre;
mod libraries;

pub struct LauncherData<D: Send + Sync> {
    pub(crate) on_stdout: fn(&D, &[u8]) -> Result<()>,
    pub(crate) on_stderr: fn(&D, &[u8]) -> Result<()>,
    pub(crate) on_progress: fn(&D, ProgressUpdate) -> Result<()>,
    pub(crate) on_log: fn(&D, &str) -> Result<()>,
    pub(crate) hide_window: fn(&D),
    pub(crate) data: Box<D>,
    pub(crate) terminator: tokio::sync::oneshot::Receiver<()>,
}

impl<D: Send + Sync> LauncherData<D> {
    fn hide_window(&self) {
        (self.hide_window)(&self.data);
    }
}

impl<D: Send + Sync> ProgressReceiver for LauncherData<D> {
    fn progress_update(&self, progress_update: ProgressUpdate) {
        let _ = (self.on_progress)(&self.data, progress_update);
    }
    fn log(&self, msg: &str) {
        let _ = (self.on_log)(&self.data, msg);
    }
}

///
/// Launches the game
///
pub async fn launch<D: Send + Sync>(
    data: &Path,
    manifest: LaunchManifest,
    version_profile: VersionProfile,
    launching_parameter: StartParameter,
    launcher_data: LauncherData<D>,
) -> Result<()> {
    let features: HashSet<String> = HashSet::new();
    let mut class_path = String::new();

    launcher_data.progress_update(ProgressUpdate::set_label("Setting up..."));

    launcher_data.log(&format!(
        "Determined OS to be {} {}",
        OS,
        OS_VERSION.clone()
    ));

    let runtimes_folder = join_and_mkdir!(data, "runtimes");
    let client_folder = join_and_mkdir_vec!(data, vec!["versions", &version_profile.id]);
    let natives_folder = join_and_mkdir!(client_folder, "natives");
    let libraries_folder = join_and_mkdir!(data, "libraries");
    let assets_folder = join_and_mkdir!(data, "assets");
    let game_dir = join_and_mkdir_vec!(data, vec!["gameDir", &*manifest.build.branch]);

    let java_bin = load_jre(
        &runtimes_folder,
        &manifest,
        &launching_parameter,
        &launcher_data,
    )
    .await
    .context("Failed to load JRE")?;

    launcher_data.log(&format!("Java Path: {:?}", java_bin));
    if !java_bin.exists() {
        bail!("Java binary not found");
    }

    // Check if json has client download (or doesn't require one)
    setup_client_jar(
        &client_folder,
        &natives_folder,
        &version_profile,
        &launcher_data,
        &mut class_path,
    )
    .await
    .context("Failed to setup client JAR")?;

    // Libraries
    setup_libraries(
        &libraries_folder,
        &natives_folder,
        &version_profile,
        &launching_parameter,
        &launcher_data,
        &features,
        &mut class_path,
    )
    .await
    .context("Failed to setup libraries")?;

    // Assets
    let asset_index_location = setup_assets(
        &assets_folder,
        &version_profile,
        &launching_parameter,
        &launcher_data,
    )
    .await
    .context("Failed to setup assets")?;

    // Game
    let java_runtime = JavaRuntime::new(java_bin);

    let mut command_arguments = Vec::new();

    // JVM Args
    version_profile.arguments.add_jvm_args_to_vec(
        &mut command_arguments,
        &launching_parameter,
        &features,
    )?;

    // Custom Arguments
    command_arguments.extend(launching_parameter.jvm_args.iter().cloned());

    // Main class
    command_arguments.push(
        version_profile
            .main_class
            .as_ref()
            .ok_or_else(|| {
                LauncherError::InvalidVersionProfile("Main class unspecified".to_string())
            })?
            .to_owned(),
    );

    // Game args
    version_profile
        .arguments
        .add_game_args_to_vec(&mut command_arguments, &features)?;

    let mut mapped: Vec<String> = Vec::with_capacity(command_arguments.len());

    for x in command_arguments.iter() {
        mapped.push(process_templates(x, |output, param| {
            match param {
                "auth_player_name" => output.push_str(&launching_parameter.auth_player_name),
                "version_name" => output.push_str(&version_profile.id),
                "game_directory" => {
                    output.push_str(game_dir.absolutize().unwrap().to_str().unwrap())
                }
                "assets_root" => {
                    output.push_str(assets_folder.absolutize().unwrap().to_str().unwrap())
                }
                "assets_index_name" => output.push_str(&asset_index_location.id),
                "auth_uuid" => output.push_str(&launching_parameter.auth_uuid),
                "auth_access_token" => output.push_str(&launching_parameter.auth_access_token),
                "user_type" => output.push_str(&launching_parameter.user_type),
                "version_type" => output.push_str(&version_profile.version_type),
                "natives_directory" => {
                    output.push_str(natives_folder.absolutize().unwrap().to_str().unwrap())
                }
                "launcher_name" => output.push_str("LiquidLauncher"),
                "launcher_version" => output.push_str(LAUNCHER_VERSION),
                "classpath" => output.push_str(&class_path),
                "user_properties" => output.push_str("{}"),
                "clientid" => output.push_str(&launching_parameter.clientid),
                "auth_xuid" => output.push_str(&launching_parameter.auth_xuid),
                _ => return Err(LauncherError::UnknownTemplateParameter(param.to_owned()).into()),
            };

            Ok(())
        })?);
    }

    launcher_data.progress_update(ProgressUpdate::set_label("Launching..."));
    launcher_data.progress_update(ProgressUpdate::set_to_max());

    let mut running_task = java_runtime.execute(mapped, &game_dir).await?;

    launcher_data.progress_update(ProgressUpdate::set_label("Running..."));

    if !launching_parameter.keep_launcher_open {
        // Hide launcher window
        launcher_data.hide_window();
    }

    let terminator = launcher_data.terminator;
    let data = launcher_data.data;

    java_runtime
        .handle_io(
            &mut running_task,
            launcher_data.on_stdout,
            launcher_data.on_stderr,
            terminator,
            &data,
        )
        .await?;

    if !launching_parameter.keep_launcher_open {
        // Hide launcher window
        exit(0);
    }

    Ok(())
}

pub struct StartParameter {
    pub java_distribution: DistributionSelection,
    pub jvm_args: Vec<String>,
    pub memory: u64,
    pub custom_data_path: Option<String>,
    pub auth_player_name: String,
    pub auth_uuid: String,
    pub auth_access_token: String,
    pub auth_xuid: String,
    pub clientid: String,
    pub user_type: String,
    pub keep_launcher_open: bool,
    pub concurrent_downloads: u32,
    pub client_account: Option<ClientAccount>,
    pub skip_advertisement: bool,
}

fn process_templates<F: Fn(&mut String, &str) -> Result<()>>(
    input: &String,
    retriever: F,
) -> Result<String> {
    let mut output = String::with_capacity(input.len() * 3 / 2);

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek().map_or(false, |&x| x == '{') {
            // Consuuuuume the '{'
            chars.next();

            let mut template_arg = String::with_capacity(input.len() - 3);

            let mut c;

            loop {
                c = chars.next().ok_or_else(|| {
                    LauncherError::InvalidVersionProfile(
                        "invalid template, missing '}'".to_string(),
                    )
                })?;

                if c == '}' {
                    break;
                }
                if !matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9') {
                    return Err(LauncherError::InvalidVersionProfile(format!(
                        "invalid character in template: '{}'",
                        c
                    ))
                    .into());
                }

                template_arg.push(c);
            }

            retriever(&mut output, template_arg.as_str())?;
            continue;
        }

        output.push(c);
    }

    Ok(output)
}
