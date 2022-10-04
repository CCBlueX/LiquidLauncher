use core::convert::AsRef;

use crate::minecraft::launcher::LauncherData;

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

#[derive(Debug)]
pub enum ProgressUpdate {
    SetMax(u64),
    SetProgress(u64),
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
        return Self::SetLabel(str.as_ref().to_owned());
    }
}

pub trait ProgressReceiver {
    fn progress_update(&self, update: ProgressUpdate);
}

