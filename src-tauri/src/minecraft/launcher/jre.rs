use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{
    app::api::LaunchManifest,
    minecraft::{
        java::{find_java_binary, jre_downloader},
        progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
    },
};

use super::{LauncherData, LaunchingParameter};

pub async fn load_jre<D: Send + Sync>(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    launching_parameter: &LaunchingParameter,
    launcher_data: &LauncherData<D>,
) -> Result<PathBuf> {
    if let Some(jre) = &launching_parameter.custom_java_path {
        return Ok(PathBuf::from(jre));
    }

    launcher_data.progress_update(ProgressUpdate::set_label("Checking for JRE..."));

    if let Ok(jre) = find_java_binary(
        runtimes_folder,
        &manifest.build.jre_distribution,
        &manifest.build.jre_version,
    )
    .await
    {
        return Ok(jre);
    }

    launcher_data.log("Downloading JRE...");
    launcher_data.progress_update(ProgressUpdate::set_label("Download JRE..."));

    jre_downloader::jre_download(
        &runtimes_folder,
        &manifest.build.jre_distribution,
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
