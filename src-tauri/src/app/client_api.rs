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
use std::future::Future;

use crate::auth::ClientAccount;
use crate::minecraft::java::{JavaDistribution, JavaPackage};
use crate::minecraft::version::LibraryPatch;
use crate::utils::{get_maven_artifact_path, ARCHITECTURE, OS};
use crate::HTTP_CLIENT;
use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::{debug, debug_span, error, info, warn};
use tokio::try_join;
use crate::app::client_api_target::CLIENT_BRANCH;

/// API endpoint url
pub const LAUNCHER_API: [&str; 3] = [
    "https://api.liquidbounce.net",
    "https://api.ccbluex.net",

    // Non-secure connection requires additional confirmation from the user,
    // as they are vulnerable to MITM attacks and data leaks.
    // A VPN or a proxy can be used to secure the connection.
    "http://nossl.api.liquidbounce.net",
];

#[derive(Serialize, Deserialize)]
pub struct Client {
    url: String,
    // To show a warning to the user when using a non-secure connection,
    // we need to pass this information to the frontend.
    is_secure: bool,
    session_token: String
}

impl Client {
    pub fn new(host: &str, session_token: String) -> Self {
        Self {
            url: host.to_string(),
            is_secure: host.starts_with("https://"),
            session_token
        }
    }

    /// Finds the first available API endpoint
    /// and returns a [Client] instance with the endpoint set.
    ///
    /// Returns [String] as error with technical information if no API endpoint is reachable.
    pub async fn lookup(session_token: String) -> Result<Self, String> {
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
                return Ok(Self::new(endpoint, session_token));
            }
        }

        // If no API endpoint is reachable, we bail with the technical information
        // as the error message because we already have 'Unable to connect to LiquidBounce API'
        // as header.
        Err(technical_information)
    }

    /// Check if the API endpoint is secure
    pub fn is_secure(&self) -> bool {
        self.is_secure
    }

    pub fn url(&self) -> &str {
        &self.url
    }
    
    pub fn session_token(&self) -> &str {
        &self.session_token
    }

    pub async fn blog_posts(&self, page: u32) -> Result<PaginatedResponse<BlogPost>> {
        self.request("blog/posts", &[("offset", &page.saturating_sub(1).saturating_mul(10).to_string()), ("limit", "10")]).await
    }

    /// A page of builds, newest first: releases, or every build with `nightly`.
    pub async fn build_page(
        &self,
        page: u32,
        limit: u32,
        nightly: bool,
    ) -> Result<PaginatedResponse<Build>> {
        let offset = page.saturating_sub(1).saturating_mul(limit).to_string();
        let limit = limit.to_string();
        let params = [("edition", CLIENT_BRANCH), ("offset", offset.as_str()), ("limit", &limit), ("release", "true")];
        self.request("builds", if nightly { &params[..3] } else { &params }).await
    }

    pub async fn build(&self, build_id: u32) -> Result<Build> {
        self.request(&format!("builds/{build_id}"), &[]).await
    }

    pub async fn java_distributions(&self) -> Result<Vec<JavaVendor>> {
        let (packages, vendors) = try_join!(
            self.request("launcher/java-packages", &[]),
            self.request("launcher/java-vendors", &[]),
        )?;
        Ok(local_distributions(packages, vendors, OS.get_simple_name()?, ARCHITECTURE.get_simple_name()?))
    }

    /// Request launch manifest of specific build
    pub async fn fetch_launch_manifest(&self, build_id: u32) -> Result<LaunchManifest> {
        self.request(&format!("builds/{build_id}/launch-manifest"), &[]).await
    }

    /// Request list of downloadable mods for mc_version and used subsystem
    pub async fn fetch_mods(&self, mc_version: &str, subsystem: &str) -> Result<Vec<LoaderMod>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Mod {
            name: String, artifact: String, repository: String, required: bool, enabled_by_default: bool,
        }
        let mods = self.request_pages::<Mod>("launcher/mods", &[("mcVersion", mc_version), ("subsystem", subsystem)]).await?;
        Ok(mods.into_iter().map(|m| LoaderMod {
            name: m.name, required: m.required, enabled: m.enabled_by_default,
            source: ModSource::Repository { artifact: m.artifact, repository: m.repository },
        }).collect())
    }

    /// Request changelog of specified build
    pub async fn fetch_changelog(&self, build_id: u32) -> Result<Changelog> {
        #[derive(Deserialize)]
        struct Detail {
            #[serde(flatten)]
            build: Build,
            changelog: Option<Text>,
        }
        #[derive(Deserialize)]
        struct Text { text: String }
        let detail: Detail = self.request(&format!("builds/{build_id}"), &[]).await?;
        let changelog = detail.changelog.map(|c| c.text).unwrap_or_else(|| detail.build.message.clone());
        Ok(Changelog { build: detail.build, changelog })
    }

    pub async fn fetch_user(&self, client_account: &ClientAccount) -> Result<UserInformation> {
        Ok(client_account.authenticate_request(HTTP_CLIENT.get(format!("{}/me", self.url)))?
            .send().await?.error_for_status()?.json().await?)
    }

    pub async fn request_pages<T: DeserializeOwned>(&self, endpoint: &str, filters: &[(&str, &str)]) -> Result<Vec<T>> {
        let base = Url::parse_with_params(&format!("{}/{endpoint}", self.url), filters)?;
        all_pages(|offset| {
            let mut url = base.clone();
            url.query_pairs_mut().append_pair("limit", "100").append_pair("offset", &offset.to_string());
            self.get(url)
        }).await
    }

    /// Request JSON formatted data from launcher API, with query parameters
    pub async fn request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let url = Url::parse_with_params(&format!("{}/{}", self.url, endpoint), query)?;
        self.get(url).await
    }

    async fn get<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        Ok(HTTP_CLIENT
            .get(url)
            .header("X-Session-Token", &self.session_token)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }
}

#[derive(Deserialize)]
struct ListedPackage {
    vendor: JavaDistribution,
    os: String,
    arch: String,
}

#[derive(Deserialize, Serialize)]
pub struct JavaVendor {
    vendor: JavaDistribution,
    name: String,
}

/// The vendors with a package for this platform, in the order the API lists them. One it has
/// no name for goes last, under its identifier.
fn local_distributions(packages: Vec<ListedPackage>, vendors: Vec<JavaVendor>, os: &str, arch: &str) -> Vec<JavaVendor> {
    let mut local: Vec<_> = packages.into_iter()
        .filter(|p| p.os == os && p.arch == arch)
        .map(|p| p.vendor)
        .collect();
    local.sort_unstable();
    local.dedup();
    let mut listed: Vec<_> = vendors.into_iter().filter(|v| local.contains(&v.vendor)).collect();
    for vendor in local {
        if !listed.iter().any(|v| v.vendor == vendor) {
            listed.push(JavaVendor { name: vendor.get_name().to_owned(), vendor });
        }
    }
    listed
}

/// Every item of a paginated list, asking for the next page by how many items came so far.
async fn all_pages<T, F>(page_at: impl Fn(usize) -> F) -> Result<Vec<T>>
where
    F: Future<Output = Result<PaginatedResponse<T>>>,
{
    let mut items = Vec::new();
    loop {
        let page = page_at(items.len()).await?;
        if page.items.is_empty() {
            return Ok(items);
        }
        items.extend(page.items);
        if items.len() >= page.total as usize {
            return Ok(items);
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u32,
}

#[derive(Serialize, Deserialize)]
pub struct BlogPost {
    #[serde(rename(serialize = "postUid", deserialize = "id"))]
    pub post_uid: String,
    pub author: String,
    pub title: String,
    pub description: String,
    #[serde(rename(deserialize = "publishedAt"))]
    pub date: DateTime<Utc>,
    #[serde(rename = "bannerText")]
    pub banner_text: Option<String>,
    #[serde(rename = "bannerImageUrl")]
    pub banner_image_url: Option<String>,
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
    #[serde(rename(serialize = "buildId", deserialize = "id"))]
    pub build_id: u32,
    #[serde(rename = "commitId")]
    pub commit_id: String,
    #[serde(rename(deserialize = "edition"))]
    pub branch: String,
    pub subsystem: String,
    #[serde(rename = "lbVersion")]
    pub lb_version: String,
    #[serde(rename = "mcVersion")]
    pub mc_version: String,
    pub release: bool,
    #[serde(rename(deserialize = "committedAt"))]
    pub date: DateTime<Utc>,
    pub message: String,
    #[serde(rename(deserialize = "downloadUrl"))]
    pub url: String,
    #[serde(rename = "jreDistribution", default)]
    pub jre_distribution: Option<JavaDistribution>,
    #[serde(rename = "jreVersion")]
    pub jre_version: u32,
    #[serde(flatten)]
    pub subsystem_specific_data: SubsystemSpecificData,
}

///
/// Subsystem-specific data
/// This can be used for any subsystem, but for now it is only implemented for Fabric.
/// It has to be turned into an Enum to be able to decide on it's own for specific data, but for now this is not required.
///
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SubsystemSpecificData {
    // Additional data
    #[serde(rename = "fabricApiVersion")]
    pub fabric_api_version: String,
    #[serde(rename = "fabricLoaderVersion")]
    pub fabric_loader_version: String,
    #[serde(rename = "kotlinVersion")]
    pub kotlin_version: String,
    #[serde(rename = "kotlinModVersion")]
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
    #[serde(default, rename = "libraryPatches")]
    pub library_patches: Vec<LibraryPatch>,
    #[serde(rename = "javaPackages", default, deserialize_with = "JavaPackage::list")]
    pub java_packages: Vec<JavaPackage>,
}

///
/// JSON struct of mod
///
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoaderMod {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    #[serde(alias = "enabledByDefault")]
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
        skip_id: String,
        #[serde(rename = "downloadUrl")]
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
        #[serde(rename = "modDirectory")]
        mod_directory: String,
    },
    #[serde(rename = "forge")]
    Forge {
        manifest: String,
        #[serde(rename = "modDirectory")]
        mod_directory: String,
    },
}

#[derive(Deserialize, Serialize, Clone)]
pub struct UserInformation {
    pub nickname: String,
    #[serde(rename = "userId", alias = "uid")]
    pub user_id: String,
    pub premium: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_value, json, to_value};
    use std::cell::RefCell;

    #[test]
    fn catalog_lists_local_distributions_as_the_api_orders_them() {
        let packages = from_value(json!([
            { "vendor": "second", "os": "linux", "arch": "x64", "major": 17 },
            { "vendor": "first", "os": "linux", "arch": "x64", "major": 21 },
            { "vendor": "second", "os": "linux", "arch": "x64", "major": 21 },
            { "vendor": "unnamed", "os": "linux", "arch": "x64", "major": 21 },
            { "vendor": "other-arch", "os": "linux", "arch": "aarch64", "major": 21 },
            { "vendor": "other-os", "os": "windows", "arch": "x64", "major": 21 }
        ])).unwrap();
        let vendors = from_value(json!([
            { "vendor": "second", "name": "Second JDK", "position": 0 },
            { "vendor": "other-os", "name": "Elsewhere JDK", "position": 1 },
            { "vendor": "first", "name": "First JDK", "position": 2 }
        ])).unwrap();
        assert_eq!(to_value(local_distributions(packages, vendors, "linux", "x64")).unwrap(), json!([
            { "vendor": "second", "name": "Second JDK" },
            { "vendor": "first", "name": "First JDK" },
            { "vendor": "unnamed", "name": "unnamed" }
        ]));
    }

    #[tokio::test]
    async fn short_pages_advance_by_items_received() {
        let asked = RefCell::new(Vec::new());
        let items = all_pages(|offset| {
            asked.borrow_mut().push(offset);
            let items = match offset { 0 => vec![1, 2], 2 => vec![3], _ => vec![] };
            async move { Ok(PaginatedResponse { items, total: 3 }) }
        }).await.unwrap();
        assert_eq!(items, [1, 2, 3]);
        assert_eq!(*asked.borrow(), [0, 2]);
    }
}
