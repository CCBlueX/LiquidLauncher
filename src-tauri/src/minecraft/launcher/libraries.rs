use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use path_absolutize::Absolutize;
use std::fmt::Write;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use tokio::fs::OpenOptions;
use backon::ExponentialBuilder;
use backon::Retryable;

use crate::{
    error::LauncherError,
    minecraft::{
        progress::{ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
        rule_interpreter,
        version::{Library, LibraryDownloadInfo, VersionProfile},
    },
    utils::{zip_extract, Architecture, OperatingSystem, ARCHITECTURE, OS},
};

use super::{LauncherData, StartParameter};

pub async fn setup_libraries<D: Send + Sync>(
    libraries_folder: &Path,
    natives_folder: &Path,
    version_profile: &VersionProfile,
    launching_parameter: &StartParameter,
    launcher_data: &LauncherData<D>,
    features: &HashSet<String>,
    class_path: &mut String,
) -> Result<()> {
    let libraries_to_download = version_profile
        .libraries
        .iter()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();
    let libraries_max = libraries_to_download.len() as u64;
    let arm64_natives = &if OS == OperatingSystem::LINUX && *ARCHITECTURE == Architecture::AARCH64 {
        linux_arm64_natives(&libraries_to_download)
    } else {
        HashMap::new()
    };

    launcher_data.progress_update(ProgressUpdate::set_label("Checking libraries..."));
    launcher_data.progress_update(ProgressUpdate::set_for_step(
        ProgressUpdateSteps::DownloadLibraries,
        0,
        libraries_max,
    ));

    let class_paths: Vec<Result<Option<String>>> =
        stream::iter(libraries_to_download.into_iter().filter_map(|library| {
            // let download_count = libraries_downloaded.clone();
            let folder_clone = libraries_folder.to_path_buf();
            let native_clone = natives_folder.to_path_buf();

            if !rule_interpreter::check_condition(&library.rules, &features).unwrap_or(false) {
                return None;
            }

            Some(async move {
                if let Some(natives) = &library.natives {
                    if let Some(required_natives) = natives.get(OS.get_simple_name()?) {
                        if let Some(classifiers) = library
                            .downloads
                            .as_ref()
                            .and_then(|x| x.classifiers.as_ref())
                        {
                            if let Some(artifact) = classifiers
                                .get(required_natives)
                                .map(LibraryDownloadInfo::from)
                            {
                                let path = (|| async { artifact.download(&library.name, folder_clone.clone(), launcher_data).await })
                                    .retry(ExponentialBuilder::default())
                                    .notify(|err, dur| {
                                        launcher_data.log(&format!(
                                            "Failed to download native library: {}. Retrying in {:?}. Error: {}",
                                            &library.name, dur, err
                                        ));
                                    })
                                    .await
                                    .with_context(|| {
                                        format!(
                                            "Failed to download native library: {}",
                                            &library.name
                                        )
                                    })?;

                                launcher_data.progress_update(ProgressUpdate::set_label(
                                    "Extracting natives...",
                                ));
                                let file = OpenOptions::new()
                                    .read(true)
                                    .open(path)
                                    .await
                                    .context("Failed to open native library")?;
                                zip_extract(file, &native_clone)
                                    .await
                                    .context("Failed to extract native library")?;
                            }
                        } else {
                            return Err(LauncherError::InvalidVersionProfile(
                                "missing classifiers, but natives required.".to_string(),
                            )
                            .into());
                        }
                    }

                    return Ok(None);
                }



                // Download regular artifact
                let artifact = match arm64_natives.get(&library.name) {
                    Some(artifact) => artifact.clone(),
                    None => library.get_library_download()?,
                };

                let path = (|| async {  artifact.download(&library.name, folder_clone.clone(), launcher_data).await })
                    .retry(ExponentialBuilder::default())
                    .notify(|err, dur| {
                        launcher_data.log(&format!(
                            "Failed to download library: {}. Retrying in {:?}. Error: {}",
                            &library.name, dur, err
                        ));
                    })
                    .await
                    .with_context(|| format!("Failed to download library: {}", &library.name))?;

                // Natives are not included in the classpath
                return if library.natives.is_none() {
                    return Ok(path.absolutize()?.to_str().map(|x| x.to_string()));
                } else {
                    Ok(None)
                };
            })
        }))
        .buffer_unordered(launching_parameter.concurrent_downloads as usize)
        .collect()
        .await;

    // Join class paths
    for x in class_paths {
        if let Some(library_path) = x? {
            write!(class_path, "{}{}", &library_path, OS.get_path_separator()?)?;
        }
    }

    launcher_data.progress_update(ProgressUpdate::set_for_step(
        ProgressUpdateSteps::DownloadLibraries,
        libraries_max,
        libraries_max,
    ));

    Ok(())
}

/// Mojang only ships x64 LWJGL natives for Linux, Maven Central has the arm64 ones.
/// Artifacts the profile already ships arm64 natives for are left alone.
fn linux_arm64_natives(libraries: &[Library]) -> HashMap<String, LibraryDownloadInfo> {
    let names = libraries
        .iter()
        .map(|library| library.name.as_str())
        .collect::<HashSet<_>>();

    libraries
        .iter()
        .filter_map(|library| {
            let (artifact, version) = library
                .name
                .strip_prefix("org.lwjgl:")?
                .strip_suffix(":natives-linux")?
                .split_once(':')?;

            let shipped = format!("org.lwjgl:{artifact}:{version}:natives-linux-arm64");
            if names.contains(shipped.as_str()) {
                return None;
            }

            let path = format!(
                "org/lwjgl/{artifact}/{version}/{artifact}-{version}-natives-linux-arm64.jar"
            );

            Some((
                library.name.clone(),
                LibraryDownloadInfo {
                    url: format!("https://repo1.maven.org/maven2/{path}"),
                    sha1: None,
                    size: None,
                    path,
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn libraries(names: &[&str]) -> Vec<Library> {
        names
            .iter()
            .map(|name| serde_json::from_value(serde_json::json!({ "name": name })).unwrap())
            .collect()
    }

    #[test]
    fn takes_linux_arm64_natives_from_maven_central() {
        let natives = linux_arm64_natives(&libraries(&[
            "org.lwjgl:lwjgl:3.4.3",
            "org.lwjgl:lwjgl:3.4.3:natives-linux",
            "org.lwjgl:lwjgl:3.4.3:natives-windows",
        ]));

        assert_eq!(natives.len(), 1);
        assert_eq!(
            natives["org.lwjgl:lwjgl:3.4.3:natives-linux"].url,
            "https://repo1.maven.org/maven2/org/lwjgl/lwjgl/3.4.3/lwjgl-3.4.3-natives-linux-arm64.jar"
        );
    }

    #[test]
    fn keeps_arm64_natives_the_profile_ships() {
        let natives = linux_arm64_natives(&libraries(&[
            "org.lwjgl:lwjgl:3.4.3:natives-linux",
            "org.lwjgl:lwjgl:3.4.3:natives-linux-arm64",
            "org.lwjgl:lwjgl-glfw:3.4.3:natives-linux",
        ]));

        assert_eq!(natives.len(), 1);
        assert!(natives.contains_key("org.lwjgl:lwjgl-glfw:3.4.3:natives-linux"));
    }
}
