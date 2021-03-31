use std::{collections::HashMap, fmt, marker::PhantomData, path::{Path, PathBuf}, str::FromStr};

use anyhow::Result;
use log::info;
use tokio::fs;
use serde::{Deserialize, Deserializer, de::{self, MapAccess, Visitor}};
use void::Void;

// https://launchermeta.mojang.com/mc/game/version_manifest.json

#[derive(Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<ManifestVersion>
}

impl VersionManifest {

    pub async fn download() -> Result<Self> {
        Ok(reqwest::get("https://launchermeta.mojang.com/mc/game/version_manifest.json").await?.json::<VersionManifest>().await?)
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
#[serde(untagged)] // TODO: Might guess from minimum_launcher_version just to be sure.
pub enum VersionProfile {
    // V14 describes the old version json used by versions below 1.12.2
    V14(V14VersionProfile),
    // V21 describes the new version json used by versions above 1.13.
    V21(V21VersionProfile)
}

#[derive(Deserialize)]
pub struct V14VersionProfile {
    #[serde(rename = "assetIndex")]
    pub asset_index_location: AssetIndexLocation,
    pub assets: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    pub downloads: Downloads,
    pub id: String,
    pub libraries: Vec<Library>,
    pub logging: Logging,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: String,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: i32,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    #[serde(rename = "type")]
    pub version_type: String
}

#[derive(Deserialize)]
pub struct V21VersionProfile {
    pub arguments: Arguments,
    #[serde(rename = "assetIndex")]
    pub asset_index_location: AssetIndexLocation,
    pub assets: String,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: Option<i32>,
    pub downloads: Downloads,
    pub id: String,
    pub libraries: Vec<Library>,
    pub logging: Logging,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: i32,
    #[serde(rename = "releaseTime")]
    pub time: String,
    #[serde(rename = "type")]
    pub version_type: String
}

impl VersionProfile {

    pub async fn load(manifest: &ManifestVersion) -> Result<Self> {
        Ok(reqwest::get(&manifest.url).await?.json::<VersionProfile>().await?)
    }
    

}

// Parsing the arguments was pain, please mojang. What in the hell did you do?
// https://github.com/serde-rs/serde/issues/723 That's why I've done a workaround using vec_argument

#[derive(Deserialize)]
pub struct Arguments {
    #[serde(deserialize_with = "vec_argument")]
    pub game: Vec<Argument>,
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
            fs::write(&asset_index, reqwest::get(&self.url).await?.bytes().await?).await?;
        }
        
        let content = &*fs::read(&asset_index).await?;
        Ok(serde_json::from_slice::<AssetIndex>(content)?)
    }

}

#[derive(Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>
}

#[derive(Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: i64
}

impl AssetObject {

    pub async fn download(&self, assets_objects_folder: impl AsRef<Path>) -> Result<()> {
        let assets_objects_folder = assets_objects_folder.as_ref().to_owned();
        let asset_folder = assets_objects_folder.join(&self.hash[0..2]);

        if let Ok(_) = fs::create_dir(&asset_folder).await {
            // created folder
        }

        let asset_path = asset_folder.join(&self.hash);
        
        if !asset_path.exists() {
            info!("downloading {}", self.hash);
            let os = reqwest::get(&*format!("http://resources.download.minecraft.net/{}/{}", &self.hash[0..2], &self.hash)).await?.bytes().await?;
            fs::write(asset_path, os).await?;
            info!("downloaded {}", self.hash);
        }
        Ok(())
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
        let os = reqwest::get(&self.url).await?.bytes().await?;
        fs::write(path, os).await?;
        info!("downloaded {}", self.url);
        Ok(())
    }

}

#[derive(Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: LibraryDownloads,
    pub natives: Option<HashMap<String, String>>,
    pub rules: Option<Vec<Rule>>
}

#[derive(Deserialize)]
pub struct Rule {
    pub action: String
}

#[derive(Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryDownload>,
    pub classifiers: Option<HashMap<String, LibraryDownload>>
}

#[derive(Deserialize)]
pub struct LibraryDownload {
    pub path: String,
    pub sha1: String,
    pub size: i64,
    pub url: String
}

impl LibraryDownload {

    pub async fn download(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_owned();
        info!("downloading {}", self.url);
        let os = reqwest::get(&self.url).await?.bytes().await?;
        fs::write(path, os).await?;
        info!("downloaded {}", self.url);
        Ok(())
    }

}


#[derive(Deserialize)]
pub struct Logging {
    // TODO: Add logging configuration
}