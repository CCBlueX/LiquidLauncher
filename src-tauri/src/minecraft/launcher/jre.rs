use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use super::{LauncherData, StartParameter};
use crate::minecraft::java::DistributionSelection;
use crate::{
    app::client_api::LaunchManifest,
    minecraft::{
        java::{find_java_binary, jre_downloader},
        progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
    },
};

pub async fn load_jre<D: Send + Sync>(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    launching_parameter: &StartParameter,
    launcher_data: &LauncherData<D>,
) -> Result<PathBuf> {
    let distribution = match &launching_parameter.java_distribution {
        DistributionSelection::Automatic(_) => &manifest.build.jre_distribution,
        DistributionSelection::Custom(path) => return Ok(PathBuf::from(path)),
        DistributionSelection::Manual(distribution) => distribution,
    };

    // Check if distribution supports JRE version
    if !distribution.supports_version(manifest.build.jre_version) {
        return Err(anyhow!(
            "The selected JRE distribution does not support the required version of Java."
        ));
    }

    launcher_data.progress_update(ProgressUpdate::set_label("Checking for JRE..."));

    if let Ok(path) =
        find_java_binary(runtimes_folder, distribution, &manifest.build.jre_version).await
    {
        return Ok(path);
    }

    launcher_data.log("Downloading JRE...");
    launcher_data.progress_update(ProgressUpdate::set_label("Download JRE..."));

    jre_downloader::jre_download(
        &runtimes_folder,
        distribution,
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
