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

use std::fmt;

use serde::Serialize;

#[derive(Debug)]
pub enum ProgressUpdateSteps {
    DownloadLiquidBounceMods,
    DownloadJRE,
    DownloadClientJar,
    DownloadLibraries,
    DownloadAssets,
}

pub fn get_progress(idx: usize, curr: u64, max: u64) -> u64 {
    idx as u64 * 100 + (curr * 100 / max.max(1))
}

pub fn get_max(len: usize) -> u64 {
    len as u64 * 100
}

impl ProgressUpdateSteps {
    fn len() -> usize {
        5
    }

    fn step_idx(&self) -> usize {
        match self {
            ProgressUpdateSteps::DownloadLiquidBounceMods => 0,
            ProgressUpdateSteps::DownloadJRE => 1,
            ProgressUpdateSteps::DownloadClientJar => 2,
            ProgressUpdateSteps::DownloadLibraries => 3,
            ProgressUpdateSteps::DownloadAssets => 4,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum ProgressUpdate {
    #[serde(rename = "max")]
    SetMax(u64),
    #[serde(rename = "progress")]
    SetProgress(u64),
    #[serde(rename = "label")]
    SetLabel(Label),
    #[serde(rename = "speed")]
    SetDownloadSpeed(u64),
}

/// The frontend translates `progress.<key>` with `values`; `Display` is the English text for logs.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "key", content = "values", rename_all = "camelCase")]
pub enum Label {
    UpdatingLauncher { version: String },
    InstallingUpdate,
    LoadingVersionManifest,
    LoadingVersionProfile,
    InstallingAddon { name: String },
    CopiedCustomMod { name: String },
    DownloadingRecommendedMod { name: String },
    OpeningModDownloadPage { name: String },
    OpeningDownloadPage { attempt: u8, attempts: u8 },
    DownloadingMod { name: String },
    LaunchingBuild { commit: String },
    SettingUp,
    CheckingJre,
    DownloadingJre,
    DownloadingClient,
    CheckingLibraries,
    DownloadingLibrary { name: String },
    ExtractingNatives,
    CheckingAssets,
    DownloadingAsset { hash: String },
    Launching,
    Running,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UpdatingLauncher { version } => write!(f, "Updating launcher to v{version}..."),
            Self::InstallingUpdate => write!(f, "Installing update..."),
            Self::LoadingVersionManifest => write!(f, "Loading version manifest..."),
            Self::LoadingVersionProfile => write!(f, "Loading version profile..."),
            Self::InstallingAddon { name } => write!(f, "Installing add-on {name}"),
            Self::CopiedCustomMod { name } => write!(f, "Copied custom mod {name}"),
            Self::DownloadingRecommendedMod { name } => write!(f, "Downloading recommended mod {name}"),
            Self::OpeningModDownloadPage { name } => write!(f, "Opening download page for mod {name}"),
            Self::OpeningDownloadPage { attempt, attempts } => {
                write!(f, "Opening download page... (Attempt {attempt}/{attempts})")
            }
            Self::DownloadingMod { name } => write!(f, "Downloading mod {name}"),
            Self::LaunchingBuild { commit } => write!(f, "Launching {commit}..."),
            Self::SettingUp => write!(f, "Setting up..."),
            Self::CheckingJre => write!(f, "Checking for JRE..."),
            Self::DownloadingJre => write!(f, "Downloading JRE..."),
            Self::DownloadingClient => write!(f, "Downloading client..."),
            Self::CheckingLibraries => write!(f, "Checking libraries..."),
            Self::DownloadingLibrary { name } => write!(f, "Downloading library {name}"),
            Self::ExtractingNatives => write!(f, "Extracting natives..."),
            Self::CheckingAssets => write!(f, "Checking assets..."),
            Self::DownloadingAsset { hash } => write!(f, "Downloading asset object {hash}"),
            Self::Launching => write!(f, "Launching..."),
            Self::Running => write!(f, "Running..."),
        }
    }
}

const PER_STEP: u64 = 1024;

impl ProgressUpdate {
    pub fn set_for_step(step: ProgressUpdateSteps, progress: u64, max: u64) -> Self {
        Self::SetProgress(step.step_idx() as u64 * PER_STEP + (progress * PER_STEP / max))
    }
    pub fn set_to_max() -> Self {
        Self::SetProgress(ProgressUpdateSteps::len() as u64 * PER_STEP)
    }
    pub fn set_max() -> Self {
        let max = ProgressUpdateSteps::len() as u64;

        Self::SetMax(max * PER_STEP)
    }
}

pub trait ProgressReceiver {
    fn progress_update(&self, update: ProgressUpdate);
    fn log(&self, msg: &str);
}
