use anyhow::{bail, Context, Result};
use path_absolutize::Absolutize;
use std::fmt::Write;
use std::path::Path;
use tokio::fs;

use crate::{
    error::LauncherError,
    minecraft::{
        progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
        version::VersionProfile,
    },
    utils::{download_file, sha1sum, OS},
};

use super::LauncherData;

pub async fn setup_client_jar<D: Send + Sync>(
    client_folder: &Path,
    natives_folder: &Path,
    version_profile: &VersionProfile,
    launcher_data: &LauncherData<D>,
    class_path: &mut String,
) -> Result<()> {
    if let Some(client_download) = version_profile
        .downloads
        .as_ref()
        .and_then(|x| x.client.as_ref())
    {
        let client_jar = client_folder.join(format!("{}.jar", &version_profile.id));

        // Add client jar to class path
        write!(
            class_path,
            "{}{}",
            &client_jar.absolutize().unwrap().to_str().unwrap(),
            OS.get_path_separator()?
        )?;

        // Download client jar
        let requires_download = if !client_jar.exists() {
            true
        } else {
            let hash = sha1sum(&client_jar)?;
            launcher_data.log(&*format!(
                "Client JAR local hash: {}, remote: {}",
                hash, client_download.sha1
            ));
            hash != client_download.sha1
        };

        if requires_download {
            launcher_data.log("Downloading client...");
            launcher_data.progress_update(ProgressUpdate::set_label("Downloading client..."));

            let retrieved_bytes = download_file(&client_download.url, |a, b| {
                launcher_data.progress_update(ProgressUpdate::set_for_step(
                    ProgressUpdateSteps::DownloadClientJar,
                    get_progress(0, a, b),
                    get_max(1),
                ));
            })
            .await?;

            fs::write(&client_jar, retrieved_bytes)
                .await
                .context("Failed to write client JAR")?;

            // After downloading, check sha1
            let hash = sha1sum(&client_jar)?;
            launcher_data.log(&*format!(
                "Client JAR local hash: {}, remote: {}",
                hash, client_download.sha1
            ));
            if hash != client_download.sha1 {
                bail!("Client JAR download failed. SHA1 mismatch.");
            }
        }

        // Natives folder
        if !natives_folder.exists() {
            fs::create_dir_all(&natives_folder)
                .await
                .context("Failed to create natives folder")?;
        }
    } else {
        return Err(LauncherError::InvalidVersionProfile(
            "No client JAR downloads were specified.".to_string(),
        )
        .into());
    }

    Ok(())
}
