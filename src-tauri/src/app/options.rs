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

use std::{collections::HashMap, path::Path};

use crate::minecraft::java::DistributionSelection;
use crate::{auth::ClientAccount, minecraft::auth::MinecraftAccount};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Serialize, Deserialize)]
pub(crate) struct Options {
    #[serde(rename = "start")]
    pub start_options: StartOptions,
    #[serde(rename = "version")]
    pub version_options: VersionOptions,
    #[serde(rename = "launcher")]
    pub launcher_options: LauncherOptions,
    #[serde(rename = "premium")]
    pub premium_options: PremiumOptions,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct StartOptions {
    #[serde(rename = "account")]
    pub minecraft_account: Option<MinecraftAccount>,
    #[serde(rename = "customDataPath", default)]
    pub custom_data_path: String,
    #[serde(rename = "javaDistribution", default)]
    pub java_distribution: DistributionSelection,
    #[serde(rename = "jvmArgs", default)]
    pub jvm_args: Option<Vec<String>>,
    #[serde(rename = "memory", default = "default_memory")]
    pub memory: u64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct VersionOptions {
    #[serde(rename = "branchName")]
    pub branch_name: Option<String>,
    #[serde(rename = "buildId", default)]
    pub build_id: i32,
    #[serde(rename = "options", default)]
    pub options: HashMap<String, BranchOptions>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LauncherOptions {
    #[serde(rename = "showNightlyBuilds")]
    pub show_nightly_builds: bool,
    #[serde(rename = "concurrentDownloads")]
    pub concurrent_downloads: u32,
    #[serde(rename = "keepLauncherOpen")]
    pub keep_launcher_open: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PremiumOptions {
    #[serde(rename = "account")]
    pub account: Option<ClientAccount>,
    #[serde(rename = "skipAdvertisement", default)]
    pub skip_advertisement: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct BranchOptions {
    #[serde(rename = "modStates", default)]
    pub mod_states: HashMap<String, bool>,
    #[serde(rename = "customModStates", default)]
    pub custom_mod_states: HashMap<String, bool>,
}

impl Options {
    pub async fn load(app_data: &Path) -> Result<Self> {
        let file_content = fs::read(app_data.join("options.json")).await?;

        if let Ok(options) = serde_json::from_slice::<Self>(&file_content) {
            return Ok(options);
        }

        if let Ok(legacy) = serde_json::from_slice::<LegacyOptions>(&file_content) {
            return Ok(Self::from_legacy(legacy));
        }

        Ok(serde_json::from_slice::<Self>(&file_content)?)
    }

    fn from_legacy(legacy: LegacyOptions) -> Self {
        Self {
            start_options: StartOptions {
                custom_data_path: legacy.custom_data_path,
                java_distribution: DistributionSelection::default(),
                minecraft_account: legacy.current_account,
                jvm_args: None, // No equivalent in legacy format
                memory: 4096,   // No equivalent in legacy format - default to 4GB
            },
            version_options: VersionOptions {
                branch_name: None, // Force recommended branch
                build_id: -1,      // Force newest
                options: legacy.branch_options,
            },
            launcher_options: LauncherOptions {
                keep_launcher_open: legacy.keep_launcher_open,
                show_nightly_builds: legacy.show_nightly_builds,
                concurrent_downloads: legacy.concurrent_downloads as u32,
            },
            premium_options: PremiumOptions {
                account: legacy.client_account,
                skip_advertisement: legacy.skip_advertisement,
            },
        }
    }

    pub async fn store(&self, app_data: &Path) -> Result<()> {
        // store the options in the file
        fs::write(app_data.join("options.json"), serde_json::to_string(&self)?).await?;
        Ok(())
    }
}

impl Default for StartOptions {
    fn default() -> Self {
        Self {
            minecraft_account: None,
            java_distribution: DistributionSelection::default(),
            custom_data_path: String::new(),
            jvm_args: None,
            memory: 4096,
        }
    }
}

impl Default for VersionOptions {
    fn default() -> Self {
        Self {
            branch_name: None,
            build_id: -1,
            options: HashMap::new(),
        }
    }
}

impl Default for LauncherOptions {
    fn default() -> Self {
        Self {
            show_nightly_builds: false,
            keep_launcher_open: false,
            concurrent_downloads: 10,
        }
    }
}

impl Default for PremiumOptions {
    fn default() -> Self {
        Self {
            account: None,
            skip_advertisement: false,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            start_options: StartOptions::default(),
            version_options: VersionOptions::default(),
            launcher_options: LauncherOptions::default(),
            premium_options: PremiumOptions::default(),
        }
    }
}

fn default_memory() -> u64 {
    4096
}

// Legacy format structure
#[derive(Deserialize)]
#[allow(unused)]
pub(crate) struct LegacyOptions {
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
    #[serde(rename = "concurrentDownloads")]
    pub concurrent_downloads: i32,
}
