use std::path::Path;

use anyhow::{Context, Result};
use tokio::spawn;
use tracing::warn;

use crate::app::client_api::LaunchManifest;
use crate::minecraft::java::{
    find_any_java, find_java, install_java, remove_old_java, DistributionSelection,
    JavaDistribution, JavaPackage, JavaRuntime,
};
use crate::minecraft::progress::{
    get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps,
};
use crate::utils::{ARCHITECTURE, OS};

pub async fn load_jre(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    selection: &DistributionSelection,
    launcher_data: &impl ProgressReceiver,
) -> Result<JavaRuntime> {
    let chosen = match selection {
        DistributionSelection::Custom(path) => return Ok(JavaRuntime::new(path.into())),
        DistributionSelection::Manual(vendor) => Some(vendor),
        DistributionSelection::Automatic(_) => None,
    };
    let runtime = managed_jre(runtimes_folder, manifest, chosen, launcher_data).await?;

    let root = runtimes_folder.to_owned();
    spawn(async move {
        if let Err(error) = remove_old_java(&root).await {
            warn!("Could not remove old Java installations: {error:#}");
        }
    });
    Ok(runtime)
}

/// The installed Java of the chosen vendor, or of any vendor when none is chosen. Installs it
/// first when there is none.
async fn managed_jre(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    chosen: Option<&JavaDistribution>,
    launcher_data: &impl ProgressReceiver,
) -> Result<JavaRuntime> {
    let major = manifest.build.jre_version;
    let packages: Vec<&JavaPackage> = manifest
        .java_packages
        .iter()
        .filter(|package| package.fits(major))
        .collect();
    let package_of = |vendor: &JavaDistribution| {
        packages
            .iter()
            .copied()
            .find(|package| &package.vendor == vendor)
    };
    let vendors: Vec<&JavaDistribution> = match chosen {
        Some(vendor) => vec![vendor],
        // What the build asks for comes first, then every vendor the API has a package of.
        None => manifest
            .build
            .jre_distribution
            .iter()
            .chain(packages.iter().map(|package| &package.vendor))
            .collect(),
    };

    launcher_data.progress_update(ProgressUpdate::set_label("Checking for JRE..."));

    for vendor in &vendors {
        if let Some(runtime) = find_java(runtimes_folder, vendor, major).await {
            // Play starts with what is installed. A newer package is for the next launch.
            if let Some(package) = package_of(vendor) {
                update_in_background(runtimes_folder, package);
            }
            return Ok(runtime);
        }
    }
    if chosen.is_none() {
        // A vendor the API stopped listing stays in use for as long as it is installed.
        if let Some(runtime) = find_any_java(runtimes_folder, major).await {
            return Ok(runtime);
        }
    }

    let package = vendors
        .iter()
        .find_map(|vendor| package_of(vendor))
        .with_context(|| {
            format!(
                "No Java {major} package for {} on {OS}-{}. Select another distribution or a custom Java executable in Settings.",
                chosen.map_or("any distribution", JavaDistribution::get_name),
                *ARCHITECTURE,
            )
        })?;

    launcher_data.log("Downloading JRE...");
    launcher_data.progress_update(ProgressUpdate::set_label("Download JRE..."));

    install_java(runtimes_folder, package, |downloaded, total| {
        launcher_data.progress_update(ProgressUpdate::set_for_step(
            ProgressUpdateSteps::DownloadJRE,
            get_progress(0, downloaded, total),
            get_max(1),
        ));
    })
    .await
}

fn update_in_background(runtimes_folder: &Path, package: &JavaPackage) {
    let root = runtimes_folder.to_owned();
    let package = package.clone();
    spawn(async move {
        if let Err(error) = install_java(&root, &package, |_, _| ()).await {
            warn!(
                "Could not update {} Java {}: {error:#}",
                package.vendor.get_name(),
                package.major
            );
        }
    });
}
