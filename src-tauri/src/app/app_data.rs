use std::{path::Path, collections::HashMap};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;
use crate::minecraft::service::Account;

fn default_concurrent_downloads() -> i32 {
    10
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LauncherOptions {
    #[serde(rename = "keepLauncherOpen")]
    pub keep_launcher_open: bool,
    #[serde(rename = "customDataPath", default)]
    pub custom_data_path: String,
    #[serde(rename = "showNightlyBuilds")]
    pub show_nightly_builds: bool,
    #[serde(rename = "memoryPercentage")]
    pub memory_percentage: i32,
    #[serde(rename = "customJavaPath", default)]
    pub custom_java_path: String,
    #[serde(rename = "customJavaArgs", default)]
    pub custom_java_args: String,
    #[serde(rename = "preferredBranch")]
    pub preferred_branch: Option<String>,
    #[serde(rename = "preferredBuild")]
    pub preferred_build: Option<i32>,
    #[serde(rename = "currentAccount")]
    pub current_account: Option<Account>,
    #[serde(rename = "modStates", default)]
    pub mod_states: HashMap<String, bool>,
    #[serde(rename = "concurrentDownloads", default = "default_concurrent_downloads")]
    pub concurrent_downloads: i32
}

impl LauncherOptions {

    pub async fn load(app_data: &Path) -> Result<Self> {
        // load the options from the file
        Ok(serde_json::from_slice::<Self>(&fs::read(app_data.join("options.json")).await?)?)
    }

    pub async fn store(&self, app_data: &Path) -> Result<()> {
        // store the options in the file
        fs::write(app_data.join("options.json"), serde_json::to_string(&self)?).await?;
        Ok(())
    }


}

impl Default for LauncherOptions {
    fn default() -> Self {
        Self {
            keep_launcher_open: false,
            custom_data_path: String::new(),
            show_nightly_builds: false,
            memory_percentage: 80, // 80% memory of computer allocated to game
            custom_java_path: String::new(),
            custom_java_args: String::new(),
            preferred_branch: None,
            preferred_build: None,
            current_account: None,
            mod_states: HashMap::new(),
            concurrent_downloads: 10
        }
    }
}