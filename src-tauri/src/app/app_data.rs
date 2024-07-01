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
 
use std::{path::Path, collections::HashMap};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;
use crate::{auth::ClientAccount, minecraft::auth::MinecraftAccount};

fn default_concurrent_downloads() -> i32 {
    10
}

#[derive(Debug, Serialize, Deserialize)]
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
    #[serde(rename = "selectedBranch")]
    pub selected_branch: Option<String>,
    #[serde(rename = "selectedBuild")]
    pub selected_build: Option<i32>,
    #[serde(rename = "clientAccount")]
    pub client_account: Option<ClientAccount>,
    #[serde(rename = "skipAdvertisement", default)]
    pub skip_advertisement: bool,
    #[serde(rename = "currentAccount")]
    pub current_account: Option<MinecraftAccount>,
    #[serde(rename = "branchOptions", default)]
    pub branch_options: HashMap<String, BranchOptions>,
    #[serde(rename = "concurrentDownloads", default = "default_concurrent_downloads")]
    pub concurrent_downloads: i32
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct BranchOptions {
    #[serde(rename = "modStates", default)]
    pub mod_states: HashMap<String, bool>,
    #[serde(rename = "customModStates", default)]
    pub custom_mod_states: HashMap<String, bool>,
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
            selected_branch: None,
            selected_build: None,
            client_account: None,
            current_account: None,
            branch_options: HashMap::new(),
            skip_advertisement: false,
            concurrent_downloads: 10
        }
    }
}