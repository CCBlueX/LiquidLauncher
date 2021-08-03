use std::collections::{HashMap, BTreeMap};
use anyhow::Result;
use serde::{Deserialize};
use crate::utils::get_maven_artifact_path;

pub const LAUNCHER_CLOUD: &str = "https://cloud.liquidbounce.net/LiquidLauncher/";
pub const LAUNCHER_API: &str = "https://api.liquidbounce.net/";

pub const SUPPORTED_CLOUD_FILE_VERSION: u32 = 1;

#[derive(Deserialize, Debug)]
pub struct ClientVersionManifest {
    pub file_version: u32,
    pub versions: Vec<LaunchTarget>,
    pub loader_versions: HashMap<String, LoaderVersion>,
    pub repositories: BTreeMap<String, String>,
}

impl ClientVersionManifest {
    pub(crate) async fn load_version_manifest() -> Result<Self> {
        Ok(reqwest::get(format!("{}{}", LAUNCHER_API, "versions")).await?.error_for_status()?.json::<ClientVersionManifest>().await?)
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ModSource {
    #[serde(rename = "skip")]
    #[serde(rename_all = "camelCase")]
    SkipAd { artifact_name: String, url: String, #[serde(default)] extract: bool },
    #[serde(rename = "repository")]
    #[serde(rename_all = "camelCase")]
    Repository { repository: String, artifact: String },
}

impl ModSource {
    pub fn get_path(&self) -> anyhow::Result<String> {
        Ok(
            match self {
                ModSource::SkipAd { artifact_name, .. } => format!("{}.jar", artifact_name),
                ModSource::Repository { repository, artifact } => get_maven_artifact_path(artifact)?
            }
        )
    }
}

#[derive(Deserialize, Debug)]
pub struct LoaderMod {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: bool,
    pub name: String,
    pub source: ModSource,
}

#[derive(Deserialize, Debug)]
pub struct LaunchTarget {
    pub name: String,
    pub mc_version: String,
    pub loader_version: String,
    pub mods: Vec<LoaderMod>,
}


#[derive(Deserialize, Debug)]
pub struct LoaderVersion {
    pub subsystem: LoaderSubsystem,
    pub launcher_manifest: String,
    pub mod_directory: String,
}

#[derive(Deserialize, Debug)]
pub enum LoaderSubsystem {
    #[serde(rename = "fabric")]
    Fabric,
    #[serde(rename = "forge")]
    Forge,
}