use std::path::Path;

use anyhow::{Context, Result};
use tokio::spawn;
use tracing::warn;

use crate::app::client_api::LaunchManifest;
use crate::minecraft::java::{DistributionSelection, JavaRuntime};
use crate::minecraft::{
    java::{find_cached_java_binary, find_unlisted_java, jre_download, remove_stale},
    progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
};

use crate::utils::{ARCHITECTURE, OS};

pub async fn load_jre(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    selection: &DistributionSelection,
    launcher_data: &impl ProgressReceiver,
) -> Result<JavaRuntime> {
    let runtime = select_jre(runtimes_folder, manifest, selection, launcher_data).await?;
    // A custom path may point into the folder, so nothing is cleaned up while one is selected.
    if !matches!(selection, DistributionSelection::Custom(_)) {
        let root = runtimes_folder.to_owned();
        spawn(async move {
            if let Err(error) = remove_stale(&root).await {
                warn!("Could not clean up Java installations: {error}");
            }
        });
    }
    Ok(runtime)
}

async fn select_jre(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    selection: &DistributionSelection,
    launcher_data: &impl ProgressReceiver,
) -> Result<JavaRuntime> {
    let major = manifest.build.jre_version;
    let preferred = match selection {
        DistributionSelection::Custom(path) => return Ok(JavaRuntime::new(path.into())),
        DistributionSelection::Manual(vendor) => Some(vendor),
        DistributionSelection::Automatic(_) => manifest.build.jre_distribution.as_ref(),
    };

    launcher_data.progress_update(ProgressUpdate::set_label("Checking for JRE..."));

    let automatic = matches!(selection, DistributionSelection::Automatic(_));
    let packages = manifest.java_packages.iter().filter(|p| p.fits(major));
    let primary = preferred.map(|v| (v, packages.clone().find(|p| &p.vendor == v)));
    let others = packages.clone()
        .filter(|p| automatic && Some(&p.vendor) != preferred)
        .map(|p| (&p.vendor, Some(p)));
    let mut first = None;
    for (vendor, package) in primary.into_iter().chain(others) {
        first = first.or(package);
        if let Ok(runtime) = find_cached_java_binary(runtimes_folder, vendor, &major).await {
            if let Some(package) =
                package.filter(|p| runtime.version().is_none_or(|old| p.version > *old))
            {
                let root = runtimes_folder.to_owned();
                let package = package.clone();
                spawn(async move {
                    if let Err(error) = jre_download(&root, &package, |_, _| ()).await {
                        warn!(
                            "Could not update {} Java {major}: {error:#}",
                            package.vendor.get_name()
                        );
                    }
                });
            }
            return Ok(runtime);
        }
    }
    if automatic {
        let listed = |name: &str| preferred.is_some_and(|v| v.get_name() == name)
            || packages.clone().any(|p| p.vendor.get_name() == name);
        if let Some(runtime) = find_unlisted_java(runtimes_folder, major, listed).await {
            return Ok(runtime);
        }
    }
    let package = first.with_context(|| format!(
        "No Java {major} package for {} on {}-{}. Select another distribution or a custom Java executable in Settings.",
        match selection { DistributionSelection::Manual(vendor) => vendor.get_name(), _ => "any distribution" },
        OS, *ARCHITECTURE,
    ))?;

    launcher_data.log("Downloading JRE...");
    launcher_data.progress_update(ProgressUpdate::set_label("Download JRE..."));

    jre_download(runtimes_folder, package, |a, b| {
        launcher_data.progress_update(ProgressUpdate::set_for_step(
            ProgressUpdateSteps::DownloadJRE,
            get_progress(0, a, b),
            get_max(1),
        ));
    })
    .await
}
