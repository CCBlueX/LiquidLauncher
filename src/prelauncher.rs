use anyhow::Result;
use crate::cloud::{LaunchTarget, LoaderVersion};
use crate::minecraft::version::{VersionManifest, VersionProfile};
use log::*;
use crate::error::LauncherError;

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