use anyhow::Result;
use env_logger::Env;
use log::*;
use minecraft::{launcher::launch, version::{VersionManifest, VersionProfile}};
use os::OS;

pub mod minecraft;
pub mod cloud;
pub mod os;

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    debug!("Launcher cloud: {}", cloud::LAUNCHER_CLOUD);
    info!("Running on {}", OS);

    info!("Loading manifest...");
    let manifest = VersionManifest::download().await?;
    let version_manifest = manifest.versions.iter()
        .find(|m| m.id.eq_ignore_ascii_case("1.13.1"))
        .expect("Expected version");
    info!("Loading version profile...");
    let version = VersionProfile::load(version_manifest).await?;

    info!("Launching {}...", version_manifest.id);
    launch(version).await?;
    Ok(())
}