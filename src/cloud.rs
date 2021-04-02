use std::collections::HashMap;
use anyhow::Result;
use serde::{Deserialize};

pub const LAUNCHER_CLOUD: &str = "https://cloud.liquidbounce.net/LiquidLauncher/";

pub const SUPPORTED_CLOUD_FILE_VERSION: u32 = 1;

#[derive(Deserialize)]
pub struct ClientVersionManifest {
    pub file_version: u32,
    pub versions: Vec<LaunchTarget>,
    pub loader_versions: HashMap<String, LoaderVersion>,
}

impl ClientVersionManifest {
    pub(crate) async fn load_version_manifest() -> Result<Self> {
        Ok(reqwest::get(format!("{}{}", LAUNCHER_CLOUD, "version_manifest.json")).await?.error_for_status()?.json::<ClientVersionManifest>().await?)
    }
}

#[derive(Deserialize)]
pub struct LaunchTarget {
    pub name: String,
    pub mc_version: String,
    pub loader_version: String,
    mod_download: String,
}


#[derive(Deserialize)]
pub struct LoaderVersion {
    pub subsystem: LoaderSubsystem,
    pub launcher_manifest: String,
    pub mod_directory: String,
}

#[derive(Deserialize)]
pub enum LoaderSubsystem {
    #[serde(rename = "fabric")]
    Fabric,
    #[serde(rename = "forge")]
    Forge,
}