use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::Result;
use backon::{Retryable, ExponentialBuilder};
use futures::{stream, StreamExt};
use tracing::error;

use crate::{
    error::LauncherError,
    join_and_mkdir,
    minecraft::{
        progress::{ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
        version::{AssetIndexLocation, VersionProfile},
    },
};

use super::{LauncherData, StartParameter};

pub async fn setup_assets<'a, D: Send + Sync>(
    assets_folder: &'a Path,
    version_profile: &'a VersionProfile,
    launching_parameter: &'a StartParameter,
    launcher_data: &'a LauncherData<D>,
) -> Result<&'a AssetIndexLocation> {
    let indexes_folder: PathBuf = join_and_mkdir!(assets_folder, "indexes");
    let objects_folder: PathBuf = join_and_mkdir!(assets_folder, "objects");

    let asset_index_location = version_profile
        .asset_index_location
        .as_ref()
        .ok_or_else(|| {
            LauncherError::InvalidVersionProfile("Asset index unspecified".to_string())
        })?;
    let asset_index = asset_index_location
        .load_asset_index(&indexes_folder)
        .await?;
    let asset_objects_to_download = asset_index
        .objects
        .values()
        .map(|x| x.to_owned())
        .collect::<Vec<_>>();
    let assets_downloaded = Arc::new(AtomicU64::new(0));
    let asset_max = asset_objects_to_download.len() as u64;

    launcher_data.progress_update(ProgressUpdate::set_label("Checking assets..."));
    launcher_data.progress_update(ProgressUpdate::set_for_step(
        ProgressUpdateSteps::DownloadAssets,
        0,
        asset_max,
    ));

    let _: Vec<Result<()>> =
        stream::iter(asset_objects_to_download.into_iter().map(|asset_object| {
            let download_count = assets_downloaded.clone();
            let folder_clone = objects_folder.clone();

            async move {
                let hash = asset_object.hash.clone();

                match (|| async { asset_object.download(folder_clone.clone(), launcher_data).await })
                    .retry(ExponentialBuilder::default())
                    .notify(|err, dur| {
                        launcher_data.log(&format!(
                            "Failed to download asset: {}. Retrying in {:?}. Error: {}",
                            hash, dur, err
                        ));
                    })
                    .await
                {
                    Ok(downloaded) => {
                        let curr = download_count.fetch_add(1, Ordering::Relaxed);

                        if downloaded {
                            // the progress bar is only being updated when an asset has been downloaded to improve speeds
                            launcher_data.progress_update(ProgressUpdate::set_for_step(
                                ProgressUpdateSteps::DownloadAssets,
                                curr,
                                asset_max,
                            ));
                        }
                    }
                    Err(err) => {
                        // We hope the asset was not important
                        error!("Unable to download asset {}: {:?}", hash, err)
                    },
                }

                Ok(())
            }
        }))
        .buffer_unordered(launching_parameter.concurrent_downloads as usize)
        .collect()
        .await;

    launcher_data.progress_update(ProgressUpdate::set_for_step(
        ProgressUpdateSteps::DownloadAssets,
        asset_max,
        asset_max,
    ));

    Ok(asset_index_location)
}
