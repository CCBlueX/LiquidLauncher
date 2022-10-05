use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};



#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LauncherOptions {
    pub keep_launcher_open: bool,
    pub show_nightly_builds: bool,
    pub memory_size: u32,
    pub custom_java_path: String,
    pub custom_java_args: String,
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

}

impl Default for LauncherOptions {
    fn default() -> Self {
        Self {
            keep_launcher_open: false,
            show_nightly_builds: false,
            memory_size: 1024,
            custom_java_path: String::new(),
            custom_java_args: String::new(),
        }
    }
}