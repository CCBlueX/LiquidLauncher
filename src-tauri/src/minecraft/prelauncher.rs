/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2023 CCBlueX
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
 
use std::path::Path;
use std::sync::{Mutex, Arc};

use anyhow::Result;
use async_zip::read::mem::ZipFileReader;
use tracing::*;
use tokio::fs;
use tokio::io::AsyncReadExt;

use crate::app::api::{LaunchManifest, LoaderSubsystem, ModSource, LoaderMod};
use crate::error::LauncherError;
use crate::app::webview::download_client;
use crate::LAUNCHER_DIRECTORY;
use crate::minecraft::launcher;
use crate::minecraft::launcher::{LauncherData, LaunchingParameter};
use crate::minecraft::progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps};
use crate::minecraft::version::{VersionManifest, VersionProfile};
use crate::utils::{download_file, get_maven_artifact_path};

///
/// Prelaunching client
///
pub(crate) async fn launch<D: Send + Sync>(launch_manifest: LaunchManifest, launching_parameter: LaunchingParameter, additional_mods: Vec<LoaderMod>, progress: LauncherData<D>, window: Arc<Mutex<tauri::Window>>) -> Result<()> {
    info!("Loading minecraft version manifest...");
    let mc_version_manifest = VersionManifest::fetch().await?;

    let build = &launch_manifest.build;
    let subsystem = &launch_manifest.subsystem;

    progress.progress_update(ProgressUpdate::set_max());
    progress.progress_update(ProgressUpdate::SetProgress(0));

    let data_directory = launching_parameter.custom_data_path
        .clone()
        .map(|x| x.into())
        .unwrap_or_else(|| LAUNCHER_DIRECTORY.data_dir().to_path_buf());

    // Copy retrieve and copy mods from manifest
    clear_mods(&data_directory, &launch_manifest).await?;
    retrieve_and_copy_mods(&data_directory, &launch_manifest, &launch_manifest.mods, &progress, &window).await?;
    retrieve_and_copy_mods(&data_directory, &launch_manifest, &additional_mods, &progress, &window).await?;

    copy_custom_mods(&data_directory, &launch_manifest, &progress).await?;

    info!("Loading version profile...");
    let manifest_url = match subsystem {
        LoaderSubsystem::Fabric { manifest, .. } => manifest
            .replace("{MINECRAFT_VERSION}", &build.mc_version)
            .replace("{FABRIC_LOADER_VERSION}", &build.subsystem_specific_data.fabric_loader_version),
        LoaderSubsystem::Forge { manifest, .. } => manifest.clone()
    };
    let mut version = VersionProfile::load(&manifest_url).await?;

    if let Some(inherited_version) = &version.inherits_from {
        let url = mc_version_manifest.versions
            .iter()
            .find(|x| &x.id == inherited_version)
            .map(|x| &x.url)
            .ok_or_else(|| LauncherError::InvalidVersionProfile(format!("unable to find inherited version manifest {}", inherited_version)))?;

        debug!("Determined {}'s download url to be {}", inherited_version, url);
        info!("Downloading inherited version {}...", inherited_version);

        let parent_version = VersionProfile::load(url).await?;

        version.merge(parent_version)?;
    }

    info!("Launching {}...", launch_manifest.build.commit_id);

    launcher::launch(&data_directory, launch_manifest, version, launching_parameter, progress, window).await?;
    Ok(())
}

pub(crate) async fn clear_mods(data: &Path, manifest: &LaunchManifest) -> Result<()> {
    let mods_path = data.join("gameDir").join(&manifest.build.branch).join("mods");

    if !mods_path.exists() {
        return Ok(());
    }

    // Clear mods directory
    let mut mods_read = fs::read_dir(&mods_path).await?;
    while let Some(entry) = mods_read.next_entry().await? {
        if entry.file_type().await?.is_file() {
            fs::remove_file(entry.path()).await?;
        }
    }
    Ok(())
}

pub async fn retrieve_and_copy_mods(data: &Path, manifest: &LaunchManifest, mods: &Vec<LoaderMod>, progress: &impl ProgressReceiver, window: &Arc<Mutex<tauri::Window>>) -> Result<()> {
    let mod_cache_path = data.join("mod_cache");
    let mods_path = data.join("gameDir").join(&manifest.build.branch).join("mods");

    fs::create_dir_all(&mod_cache_path).await?;
    fs::create_dir_all(&mods_path).await?;

    // Download and copy mods
    let max = get_max(mods.len());

    for (mod_idx, current_mod) in mods.iter().enumerate() {
        // Skip mods that are not needed
        if !current_mod.required && !current_mod.enabled {
            continue;
        }

        progress.progress_update(ProgressUpdate::set_label(format!("Downloading recommended mod {}", current_mod.name)));

        let current_mod_path = mod_cache_path.join(current_mod.source.get_path()?);

        // Do we need to download the mod?
        if !current_mod_path.exists() {
            // Make sure that the parent directory exists
            fs::create_dir_all(&current_mod_path.parent().unwrap()).await?;

            match &current_mod.source {
                ModSource::SkipAd { artifact_name: _, url, extract } => {
                    let retrieved_bytes = download_client(url, |a, b| progress.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadLiquidBounceMods, get_progress(mod_idx, a, b) as u64, max)), window).await?;

                    // Extract bytes
                    let final_file = if *extract {
                        let reader = ZipFileReader::new(retrieved_bytes).await?;

                        // Find .JAR file in archive and get index of it
                        let index_of_file_to_extract = reader.file().entries()
                            .iter()
                            .position(|x| x.entry().filename().ends_with(".jar"))
                            .ok_or_else(|| LauncherError::InvalidVersionProfile("There is no JAR in the downloaded archive".to_string()))?;
                        let entry = reader.file().entries()[index_of_file_to_extract].entry();

                        // Read file to extract
                        let mut entry_reader = reader.entry(index_of_file_to_extract).await?;

                        let mut output = Vec::with_capacity(entry.uncompressed_size() as usize);
                        entry_reader.read_to_end(&mut output).await?;

                        output
                    } else {
                        retrieved_bytes
                    };

                    fs::write(&current_mod_path, final_file).await?;
                },
                ModSource::Repository { repository, artifact } => {
                    info!("downloading mod {} from {}", artifact, repository);
                    let repository_url = manifest.repositories.get(repository).ok_or_else(|| LauncherError::InvalidVersionProfile(format!("There is no repository specified with the name {}", repository)))?;

                    let retrieved_bytes = download_file(&format!("{}{}", repository_url, get_maven_artifact_path(artifact)?), |a, b| {
                        progress.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadLiquidBounceMods, get_progress(mod_idx, a, b), max));
                    }).await?;

                    fs::write(&current_mod_path, retrieved_bytes).await?;
                }
            }
        }

        // Copy the mod.
        fs::copy(&current_mod_path, mods_path.join(format!("{}.jar", current_mod.name))).await?;
    }

    Ok(())

}

pub async fn copy_custom_mods(data: &Path, manifest: &LaunchManifest, progress: &impl ProgressReceiver) -> Result<()> {
    let mod_cache_path = data.join("custom_mods").join(format!("{}-{}", manifest.build.branch, manifest.build.mc_version));
    let mods_path = data.join("gameDir").join(&manifest.build.branch).join("mods");

    fs::create_dir_all(&mod_cache_path).await?;
    fs::create_dir_all(&mods_path).await?;

    // Copy all mods from custom_mods to mods
    let mut mods_read = fs::read_dir(&mod_cache_path).await?;
    while let Some(entry) = mods_read.next_entry().await? {
        if entry.file_type().await?.is_file() {
            progress.progress_update(ProgressUpdate::set_label(format!("Copied custom mod {}", entry.file_name().to_str().unwrap_or_default())));
            fs::copy(entry.path(), mods_path.join(entry.file_name())).await?;
        }
    }

    Ok(())

}