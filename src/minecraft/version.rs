use std::{collections::HashMap, fmt, marker::PhantomData, path::{Path, PathBuf}, str::FromStr};

use anyhow::Result;
use log::info;
use tokio::fs;
use serde::{Deserialize, Deserializer, de::{self, MapAccess, Visitor}};
use void::Void;
use os_info::{Bitness, Info};
use std::collections::HashSet;
use crate::error::LauncherError;
use crate::utils::get_maven_artifact_path;
use std::sync::Arc;
use crate::minecraft::progress::{ProgressReceiver, ProgressUpdate};

// https://launchermeta.mojang.com/mc/game/version_manifest.json

#[derive(Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<ManifestVersion>
}

impl VersionManifest {

    pub async fn download() -> Result<Self> {
        Ok(reqwest::get("https://launchermeta.mojang.com/mc/game/version_manifest.json").await?.error_for_status()?.json::<VersionManifest>().await?)
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
    pub release_time: String
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
    pub arguments: ArgumentDeclaration
}

impl VersionProfile {
    pub(crate) fn merge(&mut self, mut parent: VersionProfile) -> anyhow::Result<()> {
        Self::merge_options(&mut self.asset_index_location, parent.asset_index_location);
        Self::merge_options(&mut self.assets, parent.assets);

        Self::merge_larger(&mut self.minimum_launcher_version, parent.minimum_launcher_version);
        Self::merge_options(&mut self.downloads, parent.downloads);
        Self::merge_larger(&mut self.compliance_level, parent.compliance_level);

        self.libraries.append(&mut parent.libraries);
        Self::merge_options(&mut self.main_class, parent.main_class);
        Self::merge_options(&mut self.logging, parent.logging);

        match &mut self.arguments {
            ArgumentDeclaration::V14(v14_a) => {
                if let ArgumentDeclaration::V14(v14_b) = parent.arguments {
                    Self::merge_options(&mut v14_a.minecraft_arguments, v14_b.minecraft_arguments);
                } else {
                    return Err(LauncherError::InvalidVersionProfile("version profile inherits from incompatible profile".to_string()).into());
                }
            }
            ArgumentDeclaration::V21(v21_a) => {
                if let ArgumentDeclaration::V21(mut v21_b) = parent.arguments {
                    v21_a.arguments.game.append(&mut v21_b.arguments.game);
                    v21_a.arguments.jvm.append(&mut v21_b.arguments.jvm);
                } else {
                    return Err(LauncherError::InvalidVersionProfile("version profile inherits from incompatible profile".to_string()).into());
                }
            }
        }

        Ok(())
    }

    fn merge_options<T>(a: &mut Option<T>, b: Option<T>) {
        if !a.is_some() {
            *a = b;
        }
    }

    fn merge_larger<T>(a: &mut Option<T>, b: Option<T>) where T: Ord {
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
    pub(crate) fn add_jvm_args_to_vec(&self, command_arguments: &mut Vec<String>, features: &HashSet<String>, os_info: &Info) -> anyhow::Result<()> {
        match self {
            ArgumentDeclaration::V14(_) => command_arguments.append(&mut vec!["-Djava.library.path=${natives_directory}".to_string(), "-cp".to_string(), "${classpath}".to_string()]),
            ArgumentDeclaration::V21(decl) => {
                ArgumentDeclaration::check_rules_and_add(command_arguments, &decl.arguments.jvm, features, os_info)?;
            }
        }

        Ok(())
    }
    pub(crate) fn add_game_args_to_vec(&self, command_arguments: &mut Vec<String>, features: &HashSet<String>, os_info: &Info) -> anyhow::Result<()> {
        match self {
            ArgumentDeclaration::V14(decl) => {
                command_arguments.extend(
                    decl.minecraft_arguments
                        .as_ref()
                        .ok_or_else(|| LauncherError::InvalidVersionProfile("no game arguments specified".to_string()))?
                        .split(" ")
                        .map(ToOwned::to_owned)
                );
            },
            ArgumentDeclaration::V21(decl) => {
                ArgumentDeclaration::check_rules_and_add(command_arguments, &decl.arguments.game, features, os_info)?;
            }
        }

        Ok(())
    }

    fn check_rules_and_add(command_arguments: &mut Vec<String>, args: &Vec<Argument>, features: &HashSet<String>, os_info: &Info) -> anyhow::Result<()> {
        for argument in args {
            if let Some(rules) = &argument.rules {
                if !crate::minecraft::rule_interpreter::check_condition(rules, &features, &os_info)? {
                    continue;
                }
            }

            match &argument.value {
                super::version::ArgumentValue::SINGLE(value) => command_arguments.push(value.to_owned()),
                super::version::ArgumentValue::VEC(vec) => command_arguments.append(&mut vec.clone())
            };
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct V14ArgumentDeclaration {
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>
}

#[derive(Deserialize)]
pub struct V21ArgumentDeclaration {
    pub arguments: Arguments,
}

impl VersionProfile {
    pub async fn load(url: &String) -> Result<Self> {
        dbg!(url);
        Ok(reqwest::get(url).await?.error_for_status()?.json::<VersionProfile>().await?)
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
    pub jvm: Vec<Argument>
}

#[derive(Deserialize)]
pub struct Argument {
    pub rules: Option<Vec<Rule>>,
    pub value: ArgumentValue
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    SINGLE(String),
    VEC(Vec<String>)
}

impl FromStr for Argument {

    type Err = Void;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Argument { value: ArgumentValue::SINGLE(s.to_string()), rules: None })
    }
}



fn vec_argument<'de, D>(deserializer: D) -> Result<Vec<Argument>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "string_or_struct")] Argument);

    let v = Vec::deserialize(deserializer).unwrap();
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
            E: serde::de::Error,
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
    pub url: String
}

impl AssetIndexLocation {

    pub async fn load_asset_index(&self, assets_root: &PathBuf) -> Result<AssetIndex> {
        let asset_index = assets_root.join(format!("{}.json", &self.id));
        
        if !asset_index.exists() {
            info!("Downloading assets index of {}", self.id);
            fs::write(&asset_index, reqwest::get(&self.url).await?.error_for_status()?.bytes().await?).await?;
        }
        
        let content = &*fs::read(&asset_index).await?;
        Ok(serde_json::from_slice::<AssetIndex>(content)?)
    }

}

#[derive(Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>
}

#[derive(Deserialize, Clone)]
pub struct AssetObject {
    pub hash: String,
    pub size: i64
}

impl AssetObject {

    pub async fn download(&self, assets_objects_folder: impl AsRef<Path>, progress: Arc<impl ProgressReceiver>) -> Result<bool> {
        let assets_objects_folder = assets_objects_folder.as_ref().to_owned();
        let asset_folder = assets_objects_folder.join(&self.hash[0..2]);

        if !asset_folder.exists() {
            fs::create_dir(&asset_folder).await?;
        }

        let asset_path = asset_folder.join(&self.hash);

        return if !asset_path.exists() {
            progress.progress_update(ProgressUpdate::set_label(format!("Downloading asset object {}", self.hash)));

            info!("Downloading {}", self.hash);
            let os = reqwest::get(&*format!("http://resources.download.minecraft.net/{}/{}", &self.hash[0..2], &self.hash)).await?.error_for_status()?.bytes().await?;
            fs::write(asset_path, os).await?;
            info!("Downloaded {}", self.hash);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn download_destructing(self, assets_objects_folder: impl AsRef<Path>, progress: Arc<impl ProgressReceiver>) -> Result<bool> {
        return self.download(assets_objects_folder, progress).await;
    }

}

#[derive(Deserialize)]
pub struct Downloads {
    pub client: Option<Download>,
    pub client_mappings: Option<Download>,
    pub server: Option<Download>,
    pub server_mappings: Option<Download>,
    pub windows_server: Option<Download>
}


#[derive(Deserialize)]
pub struct Download {
    pub sha1: String,
    pub size: i64,
    pub url: String
}

impl Download {

    pub async fn download(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_owned();
        let os = reqwest::get(&self.url).await?.error_for_status()?.bytes().await?;
        fs::write(path, os).await?;
        info!("downloaded {}", self.url);
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
    pub url: Option<String>
}

impl Library  {

    pub fn get_library_download(&self) -> anyhow::Result<LibraryDownloadInfo> {
        if let Some(artifact) = self.downloads.as_ref().and_then(|x| x.artifact.as_ref()) {
            return Ok(artifact.into());
        }

        let path = get_maven_artifact_path(&self.name)?;
        let url = self.url.as_deref().unwrap_or("https://libraries.minecraft.net/");

        return Ok(
            LibraryDownloadInfo {
                url: format!("{}{}", url, path),
                sha1: None,
                size: None,
                path,
            }
        );
    }
}

#[derive(Deserialize, Clone)]
pub struct Rule {
    pub action: RuleAction,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>
}

#[derive(Deserialize, Clone)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<OSArch>,
}

#[derive(Deserialize, Clone)]
pub enum RuleAction {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "disallow")]
    Disallow
}

#[derive(Deserialize, Clone)]
pub enum OSArch {
    #[serde(rename = "x86")]
    X32,
    #[serde(rename = "x64")]
    X64
}

impl OSArch {
    pub(crate) fn is(&self, other: &Bitness) -> Result<bool> {
        return Ok(match other {
            Bitness::X32 => matches!(self, OSArch::X32),
            Bitness::X64 => matches!(self, OSArch::X64),
            _ => anyhow::bail!("failed to determine os bitness")
        });
    }
}

#[derive(Deserialize, Clone)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryArtifact>,
    pub classifiers: Option<HashMap<String, LibraryArtifact>>
}

#[derive(Deserialize, Clone)]
pub struct LibraryArtifact {
    pub path: String,
    pub sha1: String,
    pub size: i64,
    pub url: String
}

#[derive(Deserialize, Clone)]
pub struct LibraryDownloadInfo {
    pub path: String,
    pub sha1: Option<String>,
    pub size: Option<i64>,
    pub url: String
}

impl From<&LibraryArtifact> for LibraryDownloadInfo {
    fn from(artifact: &LibraryArtifact) -> Self {
        LibraryDownloadInfo {
            path: artifact.path.to_owned(),
            sha1: Some(artifact.sha1.to_owned()),
            size: Some(artifact.size),
            url: artifact.url.to_owned()
        }
    }
}

impl LibraryDownloadInfo {

    pub async fn download(&self, name: String, libraries_folder: &Path, progress: Arc<impl ProgressReceiver>) -> Result<PathBuf> {
        let path = libraries_folder.to_path_buf();
        let library_path = path.join(&self.path);
        if library_path.exists() {
            return Ok(library_path);
        }

        fs::create_dir_all(&library_path.parent().unwrap()).await?;

        progress.progress_update(ProgressUpdate::set_label(format!("Downloading library {}", name)));

        info!("Downloading {}", self.url);
        let os = reqwest::get(&self.url).await?.error_for_status()?.bytes().await?;
        fs::write(&library_path, os).await?;
        info!("Downloaded {}", self.url);
        Ok(library_path)
    }

}


#[derive(Deserialize)]
pub struct Logging {
    // TODO: Add logging configuration
}