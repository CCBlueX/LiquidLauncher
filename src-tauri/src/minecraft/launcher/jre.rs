use std::path::Path;

use anyhow::{Context, Result};
use tokio::spawn;
use tracing::warn;

use crate::app::client_api::LaunchManifest;
use crate::minecraft::java::{DistributionSelection, JavaDistribution, JavaRuntime};
use crate::minecraft::{
    java::{find_cached_java_binary, jre_download},
    progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps},
};

use crate::utils::{ARCHITECTURE, OS};

pub async fn load_jre(
    runtimes_folder: &Path,
    manifest: &LaunchManifest,
    selection: &DistributionSelection,
    launcher_data: &impl ProgressReceiver,
) -> Result<JavaRuntime> {
    let major = manifest.build.jre_version;
    let preferred = match selection {
        DistributionSelection::Custom(path) => return Ok(JavaRuntime::new(path.into())),
        DistributionSelection::Manual(vendor) => vendor,
        DistributionSelection::Automatic(_) => &manifest.build.jre_distribution,
    };

    launcher_data.progress_update(ProgressUpdate::set_label("Checking for JRE..."));

    let vendors = [
        preferred,
        &JavaDistribution::Temurin,
        &JavaDistribution::Zulu,
        &JavaDistribution::GraalVM,
    ];
    let mut first = None;
    for (index, &vendor) in vendors.iter().enumerate() {
        if index > 0
            && (matches!(selection, DistributionSelection::Manual(_))
                || vendors[..index].contains(&vendor))
        {
            continue;
        }
        let package = manifest
            .java_packages
            .iter()
            .find(|p| &p.vendor == vendor && p.fits(major));
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
    let package = first.with_context(|| format!(
        "No trusted Java {major} package for {} on {}-{}. Select a custom Java executable in Settings.",
        match selection { DistributionSelection::Manual(vendor) => vendor.get_name(), _ => "any vendor" },
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
