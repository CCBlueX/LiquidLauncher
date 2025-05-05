/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2025 CCBlueX
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

use crate::auth::ClientAccount;
use crate::minecraft::java::JavaDistribution;
use crate::utils::get_maven_artifact_path;
use crate::HTTP_CLIENT;
use anyhow::{Error, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::{debug, debug_span, error, info, warn};

/// API endpoint url
pub const LAUNCHER_API: [&str; 3] = [
    "https://api.liquidbounce.net",
    "https://api.ccbluex.net",

    // Non-secure connection requires additional confirmation from the user,
    // as they are vulnerable to MITM attacks and data leaks.
    // A VPN or a proxy can be used to secure the connection.
    "http://nossl.api.liquidbounce.net",
];

pub const API_V1: &str = "api/v1";
pub const API_V3: &str = "api/v3";

#[derive(Serialize, Deserialize)]
pub struct Client {
    url: String,
    // To show a warning to the user when using a non-secure connection,
    // we need to pass this information to the frontend.
    is_secure: bool,
}

impl Client {
    pub fn new(host: &str) -> Self {
        Self {
            url: host.to_string(),
            is_secure: host.starts_with("https://"),
        }
    }

    /// Finds the first available API endpoint
    /// and returns a [Client] instance with the endpoint set.
    ///
    /// Returns [String] as error with technical information if no API endpoint is reachable.
    pub async fn lookup() -> Result<Self, String> {
        let span = debug_span!("api_lookup");
        let _guard = span.enter();

        // LiquidLauncher will show a technical information section in the error dialog,
        // when the API endpoint is not reachable.
        // This is to help the user to understand the issue.
        let mut technical_information = String::new();

        info!(parent: &span, "Looking up available API endpoints");
        for endpoint in LAUNCHER_API.iter() {
            if !technical_information.is_empty() {
                // Add a separator between each API endpoint
                technical_information.push('\n');
            }

            // Check if the endpoint is using SSL
            let is_secure = endpoint.starts_with("https://");
            if !is_secure {
                warn!(parent: &span, "Falling back to Non-SSL '{}' endpoint.", endpoint);
            }

            // Check if the endpoint is reachable,
            // this is as soon we get a SUCCESS response from the endpoint
            // e.g. 200 OK: LiquidBounce API written in Rust using Tokio Axum - @CCBlueX (Izuna).
            let is_success = HTTP_CLIENT
                .get(*endpoint)
                .send()
                .await
                .map_err(|err| {
                    // Cast error into anyhow::Error - because it has a better representation
                    // of the error
                    let err = Into::<Error>::into(err);
                    technical_information.push_str(&format!(
                        "Failed to connect to API endpoint '{}': {:?}\n",
                        endpoint, err
                    ));
                    error!(
                        parent: &span,
                        "Failed to connect to API endpoint '{}': {:?}",
                        endpoint, err
                    );
                    err
                })
                .is_ok_and(|r| {
                    let status = r.status();
                    let is_success = status.is_success();
                    if !is_success {
                        technical_information.push_str(&format!(
                            "API endpoint '{}' returned status code: {}\n",
                            endpoint, status
                        ));
                        error!(
                            parent: &span,
                            "API endpoint '{}' returned status code: {}",
                            endpoint, status
                        );
                    }

                    is_success
                });

            if is_success {
                debug!(parent: &span, "API endpoint '{}' is available", endpoint);
                return Ok(Self::new(endpoint));
            }
        }

        // If no API endpoint is reachable, we bail with the technical information
        // as the error message, because we already have 'Unable to connect to LiquidBounce API'
        // as header.
        Err(technical_information)
    }

    /// Check if the API endpoint is secure
    pub fn is_secure(&self) -> bool {
        self.is_secure
    }

    /// Request all blog posts
    pub async fn blog_posts(&self, page: u32) -> Result<PaginatedResponse<BlogPost>> {
        self.request_from_endpoint(API_V3, &format!("blog?page={}", page)).await
    }

    /// Request all available branches
    pub async fn branches(&self) -> Result<Branches> {
        self.request_from_endpoint(API_V1, "version/branches").await
    }

    /// Request all builds of branch
    pub async fn builds_by_branch(&self, branch: &str, release: bool) -> Result<Vec<Build>> {
        self.request_from_endpoint(API_V1, &if release {
            format!("version/builds/{}/release", branch)
        } else {
            format!("version/builds/{}", branch)
        })
        .await
    }

    /// Request launch manifest of specific build
    pub async fn fetch_launch_manifest(&self, build_id: u32) -> Result<LaunchManifest> {
        self.request_from_endpoint(API_V1, &format!("version/launch/{}", build_id))
            .await
    }

    /// Request list of downloadable mods for mc_version and used subsystem
    pub async fn fetch_mods(&self, mc_version: &str, subsystem: &str) -> Result<Vec<LoaderMod>> {
        self.request_from_endpoint(API_V1, &format!("version/mods/{}/{}", mc_version, subsystem))
            .await
    }

    /// Request changelog of specified build
    pub async fn fetch_changelog(&self, build_id: u32) -> Result<Changelog> {
        self.request_from_endpoint(API_V1, &format!("version/changelog/{}", build_id))
            .await
    }

    /// Resolve direct download link from skip file pid
    pub async fn fetch_user(&self, client_account: &ClientAccount) -> Result<UserInformation> {
        self.request_with_client_account("oauth/user", client_account)
            .await
    }

    /// Resolve direct download link from skip file pid
    pub async fn resolve_skip_file(
        &self,
        client_account: &ClientAccount,
        pid: &str,
    ) -> Result<SkipFileResolve> {
        self.request_with_client_account(&format!("file/resolve/{}", pid), client_account)
            .await
    }

    /// Request JSON formatted data from launcher API
    pub async fn request_from_endpoint<T: DeserializeOwned>(&self, api_version: &str, endpoint: &str) -> Result<T> {
        Ok(HTTP_CLIENT
            .get(format!("{}/{}/{}", self.url, api_version, endpoint))
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }

    pub async fn request_with_client_account<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        client_account: &ClientAccount,
    ) -> Result<T> {
        Ok(client_account
            .authenticate_request(HTTP_CLIENT.get(format!("{}/{}/{}", self.url, API_V3, endpoint)))?
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Serialize, Deserialize)]
pub struct Pagination {
    pub current: u32,
    pub pages: u32,
    pub items: u32,
}

#[derive(Serialize, Deserialize)]
pub struct BlogPost {
    #[serde(rename(serialize = "postId"))]
    pub post_id: u32,
    #[serde(rename(serialize = "postUid"))]
    pub post_uid: String,
    pub author: String,
    pub title: String,
    pub description: String,
    pub date: NaiveDateTime,
    #[serde(rename(serialize = "bannerText"))]
    pub banner_text: String,
    #[serde(rename(serialize = "bannerImageUrl"))]
    pub banner_image_url: String,
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
/// Subsystem-specific data
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
