use crate::cloud::{ClientVersionManifest, SUPPORTED_CLOUD_FILE_VERSION, LaunchTarget};
use anyhow::{Result, anyhow};
use std::collections::{HashMap, BTreeMap};
use std::io::stdin;
use crate::minecraft::version::VersionManifest;
use env_logger::Env;
use log::*;

pub(crate) fn cli_main(mc_version: String, lb_version: String) {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("Failed to open runtime");

    let version_manifest = rt.block_on(ClientVersionManifest::load_version_manifest()).expect("Failed to download version manifest");

    if version_manifest.file_version > SUPPORTED_CLOUD_FILE_VERSION {
        eprintln!("ERROR: Unsupported version manifest");
        return;
    }

    let launch_target = match version_manifest.versions.iter().enumerate().find(|(_, x)| x.name == lb_version && x.mc_version == mc_version) {
        Some((idx, _)) => idx,
        None => {
            eprintln!("The requested version was not found.");
            eprintln!();
            eprintln!("Available versions:");

            version_manifest.versions
                .iter()
                .for_each(|x| eprintln!("{} - {}", x.mc_version, x.name));

            return;
        }
    };

    let result = rt.block_on(async move {
        run(version_manifest, launch_target).await
    });

    if let Err(e) = result {
        println!("ERROR: {}", e);
    }
}

async fn run(version_manifest: ClientVersionManifest, launch_target_index: usize) -> Result<()> {
    info!("Loading version manifest...");

    let mc_version_manifest = VersionManifest::download().await?;

    let launch_target = &version_manifest.versions[launch_target_index];

    crate::prelauncher::launch(&mc_version_manifest, launch_target, version_manifest.loader_versions.get(&launch_target.loader_version).ok_or_else(|| anyhow!("Loader was not found"))?).await?;

    Ok(())
}