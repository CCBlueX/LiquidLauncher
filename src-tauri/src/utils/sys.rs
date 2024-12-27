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

use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fmt::Display;
use sysinfo::{RefreshKind, System, SystemExt};

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Get the total memory of the system in
pub fn sys_memory() -> u64 {
    let sys = System::new_with_specifics(RefreshKind::new().with_memory());

    sys.total_memory()
}

pub const OS: OperatingSystem = if cfg!(target_os = "windows") {
    OperatingSystem::WINDOWS
} else if cfg!(target_os = "macos") {
    OperatingSystem::OSX
} else if cfg!(target_os = "linux") {
    OperatingSystem::LINUX
} else {
    OperatingSystem::UNKNOWN
};

pub const ARCHITECTURE: Architecture = if cfg!(target_arch = "x86") {
    Architecture::X86 // 32-bit
} else if cfg!(target_arch = "x86_64") {
    Architecture::X64 // 64-bit
} else if cfg!(target_arch = "arm") {
    Architecture::ARM // ARM
} else if cfg!(target_arch = "aarch64") {
    Architecture::AARCH64 // AARCH64
} else {
    Architecture::UNKNOWN // Unsupported architecture
};

pub const OS_VERSION: Lazy<String> = Lazy::new(|| os_info::get().version().to_string());

#[derive(Deserialize, PartialEq, Eq, Hash, Debug)]
pub enum OperatingSystem {
    #[serde(rename = "windows")]
    WINDOWS,
    #[serde(rename = "linux")]
    LINUX,
    #[serde(rename = "osx")]
    OSX,
    #[serde(rename = "unknown")]
    UNKNOWN,
}

#[derive(Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Architecture {
    #[serde(rename = "x86")]
    X86,
    #[serde(rename = "x64")]
    X64,
    #[serde(rename = "arm")]
    ARM,
    #[serde(rename = "aarch64")]
    AARCH64,
    #[serde(rename = "unknown")]
    UNKNOWN,
}

impl OperatingSystem {
    pub fn get_path_separator(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => ";",
            OperatingSystem::LINUX | OperatingSystem::OSX => ":",
            _ => bail!("Invalid OS"),
        })
    }

    pub fn get_simple_name(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => "windows",
            OperatingSystem::LINUX => "linux",
            OperatingSystem::OSX => "osx",
            _ => bail!("Invalid OS"),
        })
    }

    pub fn get_adoptium_name(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => "windows",
            OperatingSystem::LINUX => "linux",
            OperatingSystem::OSX => "mac",
            _ => bail!("Invalid OS"),
        })
    }

    pub fn get_graal_name(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => "windows",
            OperatingSystem::LINUX => "linux",
            OperatingSystem::OSX => "macos",
            _ => bail!("Invalid OS"),
        })
    }

    pub fn get_archive_type(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => "zip",
            OperatingSystem::LINUX | OperatingSystem::OSX => "tar.gz",
            _ => bail!("Invalid OS"),
        })
    }
}

impl Display for OperatingSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_simple_name().unwrap())
    }
}

impl Architecture {
    pub fn get_simple_name(&self) -> Result<&'static str> {
        Ok(match self {
            Architecture::X86 => "x86",
            Architecture::X64 => "x64",
            Architecture::ARM => "arm",
            Architecture::AARCH64 => "aarch64",
            _ => bail!("Invalid architecture"),
        })
    }
}

impl Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_simple_name().unwrap())
    }
}

pub fn clean_directory(
    path: &Path,
    max_age_days: u64,
) -> Result<()> {
    let now = SystemTime::now();
    let max_age = Duration::from_days(max_age_days);

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if !metadata.is_file() {
            continue;
        }

        if let Ok(modified) = metadata.modified() {
            if now.duration_since(modified)? > max_age {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    Ok(())
}