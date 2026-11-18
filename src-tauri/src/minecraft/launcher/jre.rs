use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use super::{LauncherData, StartParameter};
use crate::app::client_api::LaunchManifest;
use crate::minecraft::java::{DistributionSelection, JavaDistribution};
use crate::minecraft::{
    java::{find_cached_java_binary, jre_downloader},
    progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
};

pub async fn load_jre<D: Send + Sync>(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    launching_parameter: &StartParameter,
    launcher_data: &LauncherData<D>,
) -> Result<PathBuf> {
    let distribution = match &launching_parameter.java_distribution {
        DistributionSelection::Automatic(_) => {
            automatic_distribution(runtimes_folder, manifest, launcher_data).await?
        }
        DistributionSelection::Custom(path) => return Ok(PathBuf::from(path)),
        DistributionSelection::Manual(distribution) => distribution.clone(),
    };

    // Check if distribution supports JRE version
    if !distribution.supports_version(manifest.build.jre_version) {
        return Err(anyhow!(
            "The selected JRE distribution does not support the required version of Java."
        ));
    }

    launcher_data.progress_update(ProgressUpdate::set_label("Checking for JRE..."));

    if let Ok(path) =
        find_cached_java_binary(runtimes_folder, &distribution, &manifest.build.jre_version).await
    {
        jre_downloader::check_for_update(runtimes_folder, distribution, manifest.build.jre_version);
        return Ok(path);
    }

    launcher_data.log("Downloading JRE...");
    launcher_data.progress_update(ProgressUpdate::set_label("Download JRE..."));

    jre_downloader::jre_download(
        &runtimes_folder,
        &distribution,
        &manifest.build.jre_version,
        |a, b| {
            launcher_data.progress_update(ProgressUpdate::set_for_step(
                ProgressUpdateSteps::DownloadJRE,
                get_progress(0, a, b),
                get_max(1),
            ));
        },
    )
    .await
}

/// The API's distribution, or the first other one with a build for this OS and architecture.
async fn automatic_distribution<D: Send + Sync>(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    launcher_data: &LauncherData<D>,
) -> Result<JavaDistribution> {
    let preferred = &manifest.build.jre_distribution;
    let version = &manifest.build.jre_version;
    let distributions = [
        preferred.clone(),
        JavaDistribution::Temurin,
        JavaDistribution::Zulu,
        JavaDistribution::GraalVM,
    ];
    for distribution in &distributions {
        if distribution.supports_version(*version)
            && find_cached_java_binary(runtimes_folder, distribution, version)
                .await
                .is_ok()
        {
            return Ok(distribution.clone());
        }
    }

    for (index, distribution) in distributions.iter().enumerate() {
        if distributions[..index].contains(distribution) || !distribution.supports_version(*version)
        {
            continue;
        }
        if distribution.has_build(version).await.unwrap_or(true) {
            if distribution != preferred {
                launcher_data.log(&format!(
                    "{} has no Java {} build for this platform, using {}",
                    preferred.get_name(),
                    version,
                    distribution.get_name()
                ));
            }

            return Ok(distribution.clone());
        }
    }

    bail!(
        "No distribution has a Java {} build for this platform",
        version
    )
}
