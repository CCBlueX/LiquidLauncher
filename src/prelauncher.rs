use anyhow::Result;
use crate::cloud::{LaunchTarget, LoaderVersion, ClientVersionManifest, ModSource};
use crate::minecraft::version::{VersionManifest, VersionProfile};
use log::*;
use crate::error::LauncherError;
use std::path::Path;
use crate::utils::{download_file, get_maven_artifact_path};
use crate::webview_utils::download_client;
use std::io::{Cursor, BufReader, Read};
use std::fs;

pub(crate) async fn launch(version_manifest: &VersionManifest, target: &LaunchTarget, loader_version: &LoaderVersion) -> Result<()> {
    info!("Loading version profile...");

    let mut version = VersionProfile::load(&loader_version.launcher_manifest).await?;

    if let Some(inherited_version) = &version.inherits_from {
        let url = version_manifest.versions
            .iter()
            .find(|x| &x.id == inherited_version)
            .map(|x| &x.url)
            .ok_or_else(|| LauncherError::InvalidVersionProfile(format!("unable to find inherited version manifest {}", inherited_version)))?;

        debug!("Determined {}'s download url to be {}", inherited_version, url);
        info!("Downloading inherited version {}...", inherited_version);

        let parent_version = VersionProfile::load(&url).await?;

        version.merge(parent_version)?;
    }

    info!("Launching {}...", target.name);

    crate::minecraft::launcher::launch(version).await?;

    Ok(())
}

pub(crate) async fn retrieve_and_copy_mods(manifest: &ClientVersionManifest, target: &LaunchTarget) -> anyhow::Result<()> {
    let mod_cache_path = Path::new("mod_cache");
    let mods_path = Path::new("gameDir").join("mods");

    tokio::fs::create_dir_all(&mod_cache_path).await?;
    tokio::fs::create_dir_all(&mods_path).await?;

    // Clear mods directory
    for x in std::fs::read_dir(&mods_path)? {
        let entry = x?;

        if entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
        }
    }

    for current_mod in &target.mods {
        // Skip mods that are not needed
        if !current_mod.required && !current_mod.default {
            continue;
        }

        let current_mod_path = mod_cache_path.join(current_mod.source.get_path()?);

        // Do we need to download the mod?
        if !current_mod_path.exists() {
            // Make sure that the parent directory exists
            tokio::fs::create_dir_all(&current_mod_path.parent().unwrap()).await?;

            match &current_mod.source {
                ModSource::SkipAd { artifact_name, url, extract } => {
                    let retrieved_bytes = download_client(url, |a, b| {}).await?;

                    // Extract bytes
                    let final_file = if *extract {
                        let mut archive = zip::ZipArchive::new(Cursor::new(retrieved_bytes)).unwrap();

                        let file_name_to_extract = archive.file_names().find(|x| x.ends_with(".jar")).ok_or_else(|| LauncherError::InvalidVersionProfile(format!("There is no JAR in the downloaded archive")))?.to_owned();

                        let mut file_to_extract = archive.by_name(&file_name_to_extract)?;

                        let mut output = Vec::with_capacity(file_to_extract.size() as usize);

                        file_to_extract.read_to_end(&mut output)?;

                        output
                    } else {
                        retrieved_bytes
                    };

                    tokio::fs::write(&current_mod_path, final_file).await?;
                },
                ModSource::Repository { repository, artifact } => {
                    info!("downloading mod {} from {}", artifact, repository);
                    let repository_url = manifest.repositories.get(repository).ok_or_else(|| LauncherError::InvalidVersionProfile(format!("There is no repository specified with the name {}", repository)))?;

                    let retrieved_bytes = download_file(&format!("{}{}", repository_url, get_maven_artifact_path(artifact)?), |a, b| {}).await?;

                    tokio::fs::write(&current_mod_path, retrieved_bytes).await?;
                }
            }
        }

        // Copy the mod.
        tokio::fs::copy(&current_mod_path, mods_path.join(format!("{}.jar", current_mod.name))).await?;
    }

    Ok(())

}