/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2024 CCBlueX
 *
 * LiquidLauncher is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * LiquidLauncher is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with LiquidLauncher. If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::auth::ClientAccount;
use crate::minecraft::java::JavaDistribution;
use crate::utils::get_maven_artifact_path;
use crate::HTTP_CLIENT;

/// API endpoint url
pub const LAUNCHER_API: &str = "https://api.liquidbounce.net";
pub const API_V1: &str = "api/v1";
pub const API_V3: &str = "api/v3";

pub const CONTENT_DELIVERY: &str = "https://cloud.liquidbounce.net";
pub const CONTENT_FOLDER: &str = "LiquidLauncher";

/// Placeholder struct for content delivery implementation
pub struct ContentDelivery;

///
/// Content Delivery for our LiquidBounce services
///
/// https://cloud.liquidbounce.net/LiquidLauncher/
/// /news.json
impl ContentDelivery {
    /// Request news
    pub async fn news() -> Result<Vec<News>> {
        Self::request_from_content_delivery("news.json").await
    }

    /// Request JSON formatted data from content delivery
    pub async fn request_from_content_delivery<T: DeserializeOwned>(file: &str) -> Result<T> {
        Ok(HTTP_CLIENT
            .get(format!("{}/{}/{}", CONTENT_DELIVERY, CONTENT_FOLDER, file))
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct News {
    pub title: String,
    pub description: String,
    pub date: String,
    pub url: String,
    #[serde(rename = "bannerText")]
    pub banner_text: String,
    #[serde(rename = "bannerUrl")]
    pub banner_url: String,
}

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
    pub async fn branches() -> Result<Branches> {
        Self::request_from_endpoint("version/branches").await
    }

    /// Request all builds of branch
    pub async fn builds_by_branch(branch: &str, release: bool) -> Result<Vec<Build>> {
        Self::request_from_endpoint(&if release {
            format!("version/builds/{}/release", branch)
        } else {
            format!("version/builds/{}", branch)
        })
        .await
    }

    /// Request launch manifest of specific build
    pub async fn launch_manifest(build_id: u32) -> Result<LaunchManifest> {
        Self::request_from_endpoint(&format!("version/launch/{}", build_id)).await
    }

    /// Request list of downloadable mods for mc_version and used subsystem
    pub async fn mods(mc_version: &str, subsystem: &str) -> Result<Vec<LoaderMod>> {
        Self::request_from_endpoint(&format!("version/mods/{}/{}", mc_version, subsystem)).await
    }

    /// Request changelog of specified build
    pub async fn changelog(build_id: u32) -> Result<Changelog> {
        Self::request_from_endpoint(&format!("version/changelog/{}", build_id)).await
    }

    /// Resolve direct download link from skip file pid
    pub async fn user(client_account: &ClientAccount) -> Result<UserInformation> {
        Self::request_with_client_account("oauth/user", client_account).await
    }

    /// Resolve direct download link from skip file pid
    pub async fn resolve_skip_file(
        client_account: &ClientAccount,
        pid: &str,
    ) -> Result<SkipFileResolve> {
        Self::request_with_client_account(&format!("file/resolve/{}", pid), client_account).await
    }

    /// Request JSON formatted data from launcher API
    pub async fn request_from_endpoint<T: DeserializeOwned>(endpoint: &str) -> Result<T> {
        Ok(HTTP_CLIENT
            .get(format!("{}/{}/{}", LAUNCHER_API, API_V1, endpoint))
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }

    pub async fn request_with_client_account<T: DeserializeOwned>(
        endpoint: &str,
        client_account: &ClientAccount,
    ) -> Result<T> {
        Ok(client_account
            .authenticate_request(
                HTTP_CLIENT.get(format!("{}/{}/{}", LAUNCHER_API, API_V3, endpoint)),
            )?
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Branches {
    #[serde(rename = "defaultBranch")]
    pub default_branch: String,
    pub branches: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Changelog {
    pub build: Build,
    pub changelog: String,
}

///
/// JSON struct of Build
///
#[derive(Serialize, Deserialize, Clone)]
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
    #[serde(rename(serialize = "jreDistribution"), default)]
    pub jre_distribution: JavaDistribution,
    #[serde(rename(serialize = "jreVersion"))]
    pub jre_version: u32,
    #[serde(flatten)]
    pub subsystem_specific_data: SubsystemSpecificData,
}

///
/// Subsystem specific data
/// This can be used for any subsystem, but for now it is only implemented for Fabric.
/// It has to be turned into an Enum to be able to decide on it's own for specific data, but for now this is not required.
///
#[derive(Serialize, Deserialize, Clone)]
pub struct SubsystemSpecificData {
    // Additional data
    #[serde(rename(serialize = "fabricApiVersion"))]
    pub fabric_api_version: String,
    #[serde(rename(serialize = "fabricLoaderVersion"))]
    pub fabric_loader_version: String,
    #[serde(rename(serialize = "kotlinVersion"))]
    pub kotlin_version: String,
    #[serde(rename(serialize = "kotlinModVersion"))]
    pub kotlin_mod_version: String,
}

///
/// JSON struct of Launch Manifest
///
#[derive(Deserialize)]
pub struct LaunchManifest {
    pub build: Build,
    pub subsystem: LoaderSubsystem,
    pub mods: Vec<LoaderMod>,
    pub repositories: BTreeMap<String, String>,
}

///
/// JSON struct of mod
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoaderMod {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    #[serde(alias = "default")]
    pub enabled: bool,
    pub name: String,
    pub source: ModSource,
}

///
/// JSON struct of ModSource (the method to be used for downloading the mod)
///
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum ModSource {
    #[serde(rename = "skip")]
    #[serde(rename_all = "camelCase")]
    SkipAd {
        artifact_name: String,
        url: String,
        #[serde(default)]
        extract: bool,
    },
    #[serde(rename = "repository")]
    #[serde(rename_all = "camelCase")]
    Repository {
        repository: String,
        artifact: String,
    },
    #[serde(rename = "local")]
    #[serde(rename_all = "camelCase")]
    Local { file_name: String },
}

impl ModSource {
    pub fn get_path(&self) -> Result<String> {
        Ok(match self {
            ModSource::SkipAd { artifact_name, .. } => format!("{}.jar", artifact_name),
            ModSource::Repository {
                repository: _repository,
                artifact,
            } => get_maven_artifact_path(artifact)?,
            ModSource::Local { file_name } => file_name.clone(),
        })
    }
}

///
/// JSON struct of subsystem
///
#[derive(Deserialize)]
#[serde(tag = "name")]
pub enum LoaderSubsystem {
    #[serde(rename = "fabric")]
    Fabric {
        manifest: String,
        mod_directory: String,
    },
    #[serde(rename = "forge")]
    Forge {
        manifest: String,
        mod_directory: String,
    },
}

#[derive(Deserialize, Serialize)]
pub struct SkipFileResolve {
    pub error: bool,
    pub msg: String,
    pub target_pid: Option<String>,
    pub download_url: Option<String>,
    pub direct_url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct UserInformation {
    #[serde(rename = "userId", alias = "user_id")]
    pub user_id: String,
    pub premium: bool,
}
