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
    SetLabel(String),
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
    pub fn set_label<S: AsRef<str>>(str: S) -> Self {
        Self::SetLabel(str.as_ref().to_owned())
    }
}

pub trait ProgressReceiver {
    fn progress_update(&self, update: ProgressUpdate);
    fn log(&self, msg: &str);
}
