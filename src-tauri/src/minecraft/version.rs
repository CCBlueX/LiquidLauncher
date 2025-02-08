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

use std::{
    collections::HashMap,
    fmt,
    marker::PhantomData,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::minecraft::launcher::StartParameter;
use crate::minecraft::progress::{ProgressReceiver, ProgressUpdate};
use crate::utils::{get_maven_artifact_path, sha1sum};
use crate::{
    error::LauncherError,
    utils::{download_file_untracked, Architecture},
    HTTP_CLIENT,
};
use anyhow::{Context, Result};
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::collections::HashSet;
use tokio::fs;
use tracing::{debug, info};
use void::Void;

// https://launchermeta.mojang.com/mc/game/version_manifest.json

#[derive(Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<ManifestVersion>,
}

impl VersionManifest {
    pub async fn fetch() -> Result<Self> {
        let response = HTTP_CLIENT.get("https://launchermeta.mojang.com/mc/game/version_manifest.json")
            .send()
            .await
            .context("Connection to https://launchermeta.mojang.com/ failed. Check your internet connection.")?
            .error_for_status()
            .context("https://launchermeta.mojang.com/ returned with an error code, try again later!")?;
        let manifest = response.json::<VersionManifest>().await.context(
            "Failed to parse Version Manifest, Mojang Server responded with not valid format.",
        )?;

        Ok(manifest)
    }
}

#[derive(Deserialize)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Deserialize)]
pub struct VersionProfile {
    pub id: String,
    #[serde(rename = "assetIndex")]
    pub asset_index_location: Option<AssetIndexLocation>,
    pub assets: Option<String>,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: Option<i32>,
    pub downloads: Option<Downloads>,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: Option<i32>,
    pub libraries: Vec<Library>,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    pub logging: Option<Logging>,
    #[serde(rename = "type")]
    pub version_type: String,
    #[serde(flatten)]
    pub arguments: ArgumentDeclaration,
}

impl VersionProfile {
    pub(crate) fn merge(&mut self, parent: VersionProfile) -> Result<()> {
        Self::merge_options(&mut self.asset_index_location, parent.asset_index_location);
        Self::merge_options(&mut self.assets, parent.assets);

        Self::merge_larger(
            &mut self.minimum_launcher_version,
            parent.minimum_launcher_version,
        );
        Self::merge_options(&mut self.downloads, parent.downloads);
        Self::merge_larger(&mut self.compliance_level, parent.compliance_level);

        Self::merge_libraries(&mut self.libraries, parent.libraries);

        Self::merge_options(&mut self.main_class, parent.main_class);
        Self::merge_options(&mut self.logging, parent.logging);

        match &mut self.arguments {
            ArgumentDeclaration::V14(v14_a) => {
                if let ArgumentDeclaration::V14(v14_b) = parent.arguments {
                    Self::merge_options(&mut v14_a.minecraft_arguments, v14_b.minecraft_arguments);
                } else {
                    return Err(LauncherError::InvalidVersionProfile(
                        "version profile inherits from incompatible profile".to_string(),
                    )
                        .into());
                }
            }
            ArgumentDeclaration::V21(v21_a) => {
                if let ArgumentDeclaration::V21(mut v21_b) = parent.arguments {
                    v21_a.arguments.game.append(&mut v21_b.arguments.game);
                    v21_a.arguments.jvm.append(&mut v21_b.arguments.jvm);
                } else {
                    return Err(LauncherError::InvalidVersionProfile(
                        "version profile inherits from incompatible profile".to_string(),
                    )
                        .into());
                }
            }
        }

        Ok(())
    }

fn merge_libraries(current_libraries: &mut Vec<Library>, parent_libraries: Vec<Library>) {
    let mut library_map: HashMap<String, Library> = current_libraries
        .iter()
        .map(|lib| (lib.get_identifier(), lib.clone()))
        .collect();

    for parent_lib in parent_libraries {
        if let Some(lib) = library_map.get_mut(&parent_lib.get_identifier()) {
            lib.rules.extend(parent_lib.rules);

            if let Some(parent_downloads) = parent_lib.downloads {
                match lib.downloads.as_mut() {
                    Some(downloads) => {
                        if let Some(artifact) = parent_downloads.artifact {
                            downloads.artifact = Some(artifact);
                        }
                        if let Some(classifiers) = parent_downloads.classifiers {
                            match downloads.classifiers.as_mut() {
                                Some(lib_classifiers) => lib_classifiers.extend(classifiers),
                                None => downloads.classifiers = Some(classifiers),
                            }
                        }
                    }
                    None => lib.downloads = Some(parent_downloads),
                }
            }

            if let Some(natives) = parent_lib.natives {
                match lib.natives.as_mut() {
                    Some(lib_natives) => lib_natives.extend(natives),
                    None => lib.natives = Some(natives),
                }
            } else if lib.natives.is_none() {
                lib.natives = parent_lib.natives;
            }

            if lib.url.is_none() {
                lib.url = parent_lib.url;
            }

            continue;
        }

        library_map.insert(parent_lib.get_identifier(), parent_lib);
    }

    *current_libraries = library_map.into_values().collect();
}

    fn merge_options<T>(a: &mut Option<T>, b: Option<T>) {
        if !a.is_some() {
            *a = b;
        }
    }

    fn merge_larger<T>(a: &mut Option<T>, b: Option<T>)
    where
        T: Ord,
    {
        if let Some((val_a, val_b)) = a.as_ref().zip(b.as_ref()) {
            if val_a < val_b {
                *a = b;
            }
        } else if !a.is_some() {
            *a = b;
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)] // TODO: Might guess from minimum_launcher_version just to be sure.
pub enum ArgumentDeclaration {
    /// V21 describes the new version json used by versions above 1.13.
    V21(V21ArgumentDeclaration),
    /// V14 describes the old version json used by versions below 1.12.2
    V14(V14ArgumentDeclaration),
}

impl ArgumentDeclaration {
    pub(crate) fn add_jvm_args_to_vec(
        &self,
        command_arguments: &mut Vec<String>,
        parameter: &StartParameter,
        features: &HashSet<String>,
    ) -> Result<()> {
        command_arguments.push(format!("-Xmx{}M", parameter.memory));
        command_arguments.push("-XX:+UnlockExperimentalVMOptions".to_string());
        command_arguments.push("-XX:+UseG1GC".to_string());
        command_arguments.push("-XX:G1NewSizePercent=20".to_string());
        command_arguments.push("-XX:G1ReservePercent=20".to_string());
        command_arguments.push("-XX:MaxGCPauseMillis=50".to_string());
        command_arguments.push("-XX:G1HeapRegionSize=32M".to_string());

        match self {
            ArgumentDeclaration::V14(_) => command_arguments.append(&mut vec![
                "-Djava.library.path=${natives_directory}".to_string(),
                "-cp".to_string(),
                "${classpath}".to_string(),
            ]),
            ArgumentDeclaration::V21(decl) => {
                ArgumentDeclaration::check_rules_and_add(
                    command_arguments,
                    &decl.arguments.jvm,
                    features,
                )?;
            }
        }

        Ok(())
    }
    pub(crate) fn add_game_args_to_vec(
        &self,
        command_arguments: &mut Vec<String>,
        features: &HashSet<String>,
    ) -> Result<()> {
        match self {
            ArgumentDeclaration::V14(decl) => {
                command_arguments.extend(
                    decl.minecraft_arguments
                        .as_ref()
                        .ok_or_else(|| {
                            LauncherError::InvalidVersionProfile(
                                "no game arguments specified".to_string(),
                            )
                        })?
                        .split(" ")
                        .map(ToOwned::to_owned),
                );
            }
            ArgumentDeclaration::V21(decl) => {
                ArgumentDeclaration::check_rules_and_add(
                    command_arguments,
                    &decl.arguments.game,
                    features,
                )?;
            }
        }

        Ok(())
    }

    fn check_rules_and_add(
        command_arguments: &mut Vec<String>,
        args: &Vec<Argument>,
        features: &HashSet<String>,
    ) -> Result<()> {
        for argument in args {
            if let Some(rules) = &argument.rules {
                if !crate::minecraft::rule_interpreter::check_condition(rules, &features)? {
                    continue;
                }
            }

            match &argument.value {
                ArgumentValue::SINGLE(value) => command_arguments.push(value.to_owned()),
                ArgumentValue::VEC(vec) => command_arguments.append(&mut vec.clone()),
            };
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct V14ArgumentDeclaration {
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
}

#[derive(Deserialize)]
pub struct V21ArgumentDeclaration {
    pub arguments: Arguments,
}

impl VersionProfile {
    pub async fn load(url: &String) -> Result<Self> {
        debug!("Loading version profile from {}", url);

        let version_profile = HTTP_CLIENT
            .get(url)
            .send()
            .await
            .context(format!("failed to pull version profile from {}", url))?
            .error_for_status()
            .context(format!("{} responded with error code.", url))?
            .json::<VersionProfile>()
            .await
            .context(format!("{} responded with not valid format.", url))?;

        Ok(version_profile)
    }
}

// Parsing the arguments was pain, please mojang. What in the hell did you do?
// https://github.com/serde-rs/serde/issues/723 That's why I've done a workaround using vec_argument

#[derive(Deserialize)]
pub struct Arguments {
    #[serde(default)]
    #[serde(deserialize_with = "vec_argument")]
    pub game: Vec<Argument>,
    #[serde(default)]
    #[serde(deserialize_with = "vec_argument")]
    pub jvm: Vec<Argument>,
}

#[derive(Deserialize)]
pub struct Argument {
    pub rules: Option<Vec<Rule>>,
    pub value: ArgumentValue,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    SINGLE(String),
    VEC(Vec<String>),
}

impl FromStr for Argument {
    type Err = Void;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Argument {
            value: ArgumentValue::SINGLE(s.to_string()),
            rules: None,
        })
    }
}

fn vec_argument<'de, D>(deserializer: D) -> Result<Vec<Argument>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "string_or_struct")] Argument);

    let v = Vec::deserialize(deserializer)?;
    Ok(v.into_iter().map(|Wrapper(a)| a).collect())
}

fn string_or_struct<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Void>,
    D: Deserializer<'de>,
{
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Void>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }

        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

#[derive(Deserialize)]
pub struct AssetIndexLocation {
    pub id: String,
    pub sha1: String,
    pub size: i64,
    #[serde(rename = "totalSize")]
    pub total_size: i64,
    pub url: String,
}

impl AssetIndexLocation {
    pub async fn load_asset_index(&self, assets_root: &PathBuf) -> Result<AssetIndex> {
        let asset_index = assets_root.join(format!("{}.json", &self.id));

        if !asset_index.exists() {
            info!("Downloading assets index of {}", self.id);
            download_file_untracked(&self.url, &asset_index).await?;
            info!("Downloaded {}", self.url);
        }

        let content = &*fs::read(&asset_index).await?;
        Ok(serde_json::from_slice::<AssetIndex>(content)?)
    }
}

#[derive(Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Deserialize, Clone)]
pub struct AssetObject {
    pub hash: String,
    pub size: i64,
}

impl AssetObject {
    pub async fn download(
        &self,
        assets_objects_folder: impl AsRef<Path>,
        progress: &impl ProgressReceiver,
    ) -> Result<bool> {
        let assets_objects_folder = assets_objects_folder.as_ref().to_owned();
        let asset_folder = assets_objects_folder.join(&self.hash[0..2]);

        if !asset_folder.exists() {
            fs::create_dir(&asset_folder).await?;
        }

        let asset_path = asset_folder.join(&self.hash);

        if !asset_path.exists() {
            progress.progress_update(ProgressUpdate::set_label(format!(
                "Downloading asset object {}",
                self.hash
            )));

            info!("Downloading {}", self.hash);
            download_file_untracked(
                &*format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &self.hash[0..2],
                    &self.hash
                ),
                asset_path,
            )
            .await?;
            info!("Downloaded {}", self.hash);

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Deserialize)]
pub struct Downloads {
    pub client: Option<Download>,
    pub client_mappings: Option<Download>,
    pub server: Option<Download>,
    pub server_mappings: Option<Download>,
    pub windows_server: Option<Download>,
}

#[derive(Deserialize)]
pub struct Download {
    pub sha1: String,
    pub size: i64,
    pub url: String,
}

impl Download {
    pub async fn download(&self, path: impl AsRef<Path>) -> Result<()> {
        download_file_untracked(&self.url, path).await?;
        info!("Downloaded {}", self.url);
        Ok(())
    }
}

#[derive(Deserialize, Clone)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    pub url: Option<String>,
}

impl Library {

    fn get_identifier(&self) -> String {
        let parts: Vec<&str> = self.name.split(':').collect();
        match parts.len() {
            3 => {
                // Standard format: group:name:version
                format!("{}:{}", parts[0], parts[1])
            }
            4 => {
                // Format with classifier: group:name:version:classifier
                format!("{}:{}:{}", parts[0], parts[1], parts[3])
            }
            _ => {
                // Fallback for unexpected formats - use the whole name
                self.name.clone()
            }
        }
    }

    pub fn get_library_download(&self) -> Result<LibraryDownloadInfo> {
        if let Some(artifact) = self.downloads.as_ref().and_then(|x| x.artifact.as_ref()) {
            return Ok(artifact.into());
        }

        let path = get_maven_artifact_path(&self.name)?;
        let url = self
            .url
            .as_deref()
            .unwrap_or("https://libraries.minecraft.net/");

        Ok(LibraryDownloadInfo {
            url: format!("{}{}", url, path),
            sha1: None,
            size: None,
            path,
        })
    }
}

#[derive(Deserialize, Clone)]
pub struct Rule {
    pub action: RuleAction,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Deserialize, Clone)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<Architecture>,
}

#[derive(Deserialize, Clone)]
pub enum RuleAction {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "disallow")]
    Disallow,
}

#[derive(Deserialize, Clone)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
    pub classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[derive(Deserialize, Clone)]
pub struct LibraryArtifact {
    pub path: String,
    pub sha1: String,
    pub size: i64,
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct LibraryDownloadInfo {
    pub path: String,
    pub sha1: Option<String>,
    pub size: Option<i64>,
    pub url: String,
}

impl From<&LibraryArtifact> for LibraryDownloadInfo {
    fn from(artifact: &LibraryArtifact) -> Self {
        LibraryDownloadInfo {
            path: artifact.path.to_owned(),
            sha1: Some(artifact.sha1.to_owned()),
            size: Some(artifact.size),
            url: artifact.url.to_owned(),
        }
    }
}

impl LibraryDownloadInfo {
    async fn fetch_sha1(&self) -> Result<String> {
        HTTP_CLIENT
            .get(&format!("{}{}", &self.url, ".sha1"))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
            .context("Failed to fetch SHA1 of library")
    }

    pub async fn download(
        &self,
        name: &str,
        libraries_folder: PathBuf,
        progress: &impl ProgressReceiver,
    ) -> Result<PathBuf> {
        let library_path = libraries_folder.join(&self.path);
        let parent = library_path
            .parent()
            .context("Failed to get parent of library path")?;

        // Create parent directories
        fs::create_dir_all(parent)
            .await
            .context("Failed to create parent directories for library")?;

        // SHA1
        let sha1 = if let Some(sha1) = &self.sha1 {
            Some(sha1.clone())
        } else {
            // Check if sha1 file exists
            let sha1_path = library_path.with_extension("sha1");

            // Fetch sha1 file
            if sha1_path.exists() {
                Some(fs::read_to_string(&sha1_path).await?)
            } else {
                // If sha1 file doesn't exist, fetch it
                progress.log(&format!("Fetching SHA1 of library {}", name));
                let sha1 = self.fetch_sha1().await.map(Some).unwrap_or(None);

                // Write sha1 file
                if let Some(sha1) = &sha1 {
                    fs::write(&sha1_path, &sha1).await?;
                }
                sha1
            }
        };

        // Check if library already exists
        if library_path.exists() {
            // Check if sha1 matches
            let hash = sha1sum(&library_path).context("Failed to calculate SHA1 of library")?;

            if let Some(sha1) = &sha1 {
                if hash == *sha1 {
                    // If sha1 matches, return
                    progress.log(&format!(
                        "Library {} already exists and SHA1 matches.",
                        name
                    ));
                    return Ok(library_path);
                }
            } else {
                // If sha1 is not available, assume it matches
                progress.log(&format!("Library {} already exists.", name));
                return Ok(library_path);
            }

            // If SHA1 doesn't match, remove the file
            progress.log(&format!(
                "Library {} already exists but sha1 does not match.",
                name
            ));
            fs::remove_file(&library_path)
                .await
                .context("Failed to remove library file")?;
        }

        // Download library
        progress.progress_update(ProgressUpdate::set_label(format!(
            "Downloading library {}",
            name
        )));
        progress.log(&format!(
            "Downloading library {} (sha1: {:?}, size: {:?}) from {} to {:}",
            name,
            &self.sha1,
            &self.size,
            &self.url,
            &library_path.display()
        ));

        download_file_untracked(&self.url, &library_path)
            .await
            .context("Failed to download library")?;

        // After downloading, check SHA1
        if let Some(sha1) = &sha1 {
            let hash = sha1sum(&library_path).context("Failed to calculate SHA1 of library")?;
            if hash != *sha1 {
                anyhow::bail!("SHA1 of library {} does not match.", name);
            }
        }

        Ok(library_path)
    }
}

#[derive(Deserialize)]
pub struct Logging {
    // TODO: Add logging configuration
}
