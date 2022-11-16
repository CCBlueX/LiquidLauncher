use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;

use crate::utils::get_maven_artifact_path;

/// API endpoint url
pub const LAUNCHER_API: &str = "https://api.liquidbounce.net";
pub const LAUNCHER_API_VERSION: &str = "api/v1";

/// Placeholder struct for API endpoints implementation
pub struct ApiEndpoints;

///
/// API Endpoints of LiquidBounce API v1
/// https://api.liquidbounce.net/api/v1/
///
/// /version is being used for launcher related endpoints
/// Supports
///     /builds
///     /builds/{branch}
///     /branches
///     /launch/{build_id}
///     /mods/{mc_version}/{subsystem}
///     /jre/{os_name}/
///
impl ApiEndpoints {

    /// Request all available branches
    pub async fn branches() -> Result<Vec<String>> {
        Self::request_from_endpoint("version/branches").await
    }

    /// Request all builds (use rarely!)
    pub async fn builds() -> Result<Vec<Build>> {
        Self::request_from_endpoint("version/builds").await
    }

    /// Request all builds of branch
    pub async fn builds_by_branch(branch: String) -> Result<Vec<Build>> {
        Self::request_from_endpoint(&format!("version/builds/{}", branch)).await
    }

    /// Request launch manifest of specific build
    pub async fn launch_manifest(build_id: i32) -> Result<LaunchManifest> {
        Self::request_from_endpoint(&format!("version/launch/{}", build_id)).await
    }

    /// Request list of downloadable mods for mc_version and used subsystem
    pub async fn mods(mc_version: String, subsystem: String) -> Result<Vec<LoaderMod>> {
        Self::request_from_endpoint(&format!("version/mods/{}/{}", mc_version, subsystem)).await
    }

    /// Request download of specified JRE for specific OS and architecture
    pub async fn jre(os_name: &String, os_arch: &String, jre_version: u32) -> Result<JreSource> {
        Self::request_from_endpoint(&format!("version/jre/{}/{}/{}", os_name, os_arch, jre_version)).await
    }

    /// Request JSON formatted data from launcher API
    pub async fn request_from_endpoint<T: DeserializeOwned>(endpoint: &str) -> Result<T> {
        Ok(reqwest::get(format!("{}/{}/{}", LAUNCHER_API, LAUNCHER_API_VERSION, endpoint)).await?
            .error_for_status()?
            .json::<T>()
            .await?
        )
    }

}

///
/// JSON struct of Build
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(rename(serialize = "buildId"))]
    pub build_id: u32,
    #[serde(rename(serialize = "commitId"))]
    pub commit_id: String,
    pub branch: String,
    pub subsystem: String,
    #[serde(rename(serialize = "lbVersion"))]
    pub lb_version: String,
    #[serde(rename(serialize = "mcVersion"))]
    pub mc_version: String,
    pub release: bool,
    pub date: DateTime<Utc>,
    pub message: String,
    pub url: String,
    #[serde(rename(serialize = "jreVersion"))]
    pub jre_version: u32,
    #[serde(flatten)]
    pub subsystem_specific_data: SubsystemSpecificData
}

///
/// Subsystem specific data
/// This can be used for any subsystem, but for now it is only implemented for Fabric.
/// It has to be turned into a Enum to be able to decide on it's own for specific data, but for now this is not required.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct SubsystemSpecificData {
    // Additional data
    #[serde(rename(serialize = "fabricApiVersion"))]
    pub fabric_api_version: String,
    #[serde(rename(serialize = "fabricLoaderVersion"))]
    pub fabric_loader_version: String,
    #[serde(rename(serialize = "kotlinVersion"))]
    pub kotlin_version: String,
    #[serde(rename(serialize = "kotlinModVersion"))]
    pub kotlin_mod_version: String
}


///
/// JSON struct of Launch Manifest
///
#[derive(Deserialize, Debug)]
pub struct LaunchManifest {
    pub build: Build,
    pub subsystem: LoaderSubsystem,
    pub mods: Vec<LoaderMod>,
    pub repositories: BTreeMap<String, String>,
}

///
/// JSON struct of mod
///
#[derive(Deserialize, Debug)]
pub struct LoaderMod {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: bool,
    pub name: String,
    pub source: ModSource,
}

///
/// JSON struct of ModSource (the method to be used for downloading the mod)
///
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
    pub fn get_path(&self) -> Result<String> {
        Ok(
            match self {
                ModSource::SkipAd { artifact_name, .. } => format!("{}.jar", artifact_name),
                ModSource::Repository { repository: _repository, artifact } => get_maven_artifact_path(artifact)?
            }
        )
    }
}

///
/// JSON struct of subsystem
///
#[derive(Deserialize, Debug)]
#[serde(tag = "name")]
pub enum LoaderSubsystem {
    #[serde(rename = "fabric")]
    Fabric { manifest: String, mod_directory: String },
    #[serde(rename = "forge")]
    Forge { manifest: String, mod_directory: String  },
}

///
/// JSON struct of JRE source
///
#[derive(Deserialize)]
pub struct JreSource {
    pub version: u32,
    pub download_url: String
}
