use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::minecraft::service::Account;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LauncherOptions {
    #[serde(rename = "keepLauncherOpen")]
    pub keep_launcher_open: bool,
    #[serde(rename = "showNightlyBuilds")]
    pub show_nightly_builds: bool,
    #[serde(rename = "memorySize")]
    pub memory_size: u32,
    #[serde(rename = "customJavaPath")]
    pub custom_java_path: String,
    #[serde(rename = "customJavaArgs")]
    pub custom_java_args: String,
    #[serde(rename = "preferredBranch")]
    pub preferred_branch: Option<String>,
    #[serde(rename = "preferredBuild")]
    pub preferred_build: Option<String>,
    // todo: might move into it's own file when there is support for multiple accounts which might sync up with the used client
    #[serde(rename = "currentAccount")]
    pub current_account: Option<Account>,
}

impl LauncherOptions {

    pub fn load(app_data: &Path) -> Result<Self> {
        // load the options from the file
        Ok(serde_json::from_slice::<Self>(&*fs::read(app_data.join("options.json"))?)?)
    }

    pub fn store(&self, app_data: &Path) -> Result<()> {
        // store the options in the file
        fs::write(app_data.join("options.json"), serde_json::to_string(&self)?)?;
        Ok(())
    }


    // used for nodejs app
    pub fn from_json(json: String) -> Result<Self> {
        Ok(serde_json::from_str::<Self>(&*json)?)
    }

    // used for nodejs app
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }

}

impl Default for LauncherOptions {
    fn default() -> Self {
        Self {
            keep_launcher_open: false,
            show_nightly_builds: false,
            memory_size: 1024,
            custom_java_path: String::new(),
            custom_java_args: String::new(),
            preferred_branch: None,
            preferred_build: None,
            current_account: None
        }
    }
}