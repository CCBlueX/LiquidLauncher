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
use std::env::consts;
use std::fmt::Display;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Get the total memory of the system in
pub fn sys_memory() -> u64 {
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()),
    );

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

/// The machine's architecture, which picks the JRE and natives for the game. An emulated launcher
/// (x64 under Rosetta or on Windows ARM) still gets a native game.
pub static ARCHITECTURE: Lazy<Architecture> =
    Lazy::new(|| match native_arch().unwrap_or(consts::ARCH) {
        "x86" => Architecture::X86,         // 32-bit
        "x86_64" => Architecture::X64,      // 64-bit
        "arm" => Architecture::ARM,         // ARM
        "aarch64" => Architecture::AARCH64, // AARCH64
        _ => Architecture::UNKNOWN,         // Unsupported architecture
    });

#[cfg(target_os = "macos")]
fn native_arch() -> Option<&'static str> {
    use sysctl::Sysctl;

    // Set on Apple Silicon, also for processes Rosetta translates
    let arm64 = sysctl::Ctl::new("hw.optional.arm64").ok()?.value().ok()?;
    matches!(arm64, sysctl::CtlValue::Int(1)).then_some("aarch64")
}

#[cfg(windows)]
fn native_arch() -> Option<&'static str> {
    use windows_sys::Win32::System::SystemInformation::{
        IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_ARM64, IMAGE_FILE_MACHINE_I386,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};

    // GetNativeSystemInfo would report x64 to an x64 process emulated on ARM64
    let (mut process, mut native) = (0, 0);
    if unsafe { IsWow64Process2(GetCurrentProcess(), &mut process, &mut native) } == 0 {
        return None;
    }

    match native {
        IMAGE_FILE_MACHINE_ARM64 => Some("aarch64"),
        IMAGE_FILE_MACHINE_AMD64 => Some("x86_64"),
        IMAGE_FILE_MACHINE_I386 => Some("x86"),
        _ => None,
    }
}

// FEX and box64 hide the machine from emulated processes, Linux ARM gets native builds instead
#[cfg(not(any(target_os = "macos", windows)))]
fn native_arch() -> Option<&'static str> {
    None
}

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
    #[serde(rename = "unknown", other)]
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

    pub fn get_zulu_name(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => "windows",
            OperatingSystem::LINUX => "linux_glibc",
            OperatingSystem::OSX => "macos",
            _ => bail!("Unsupported operating system for Zulu runtime"),
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
    pub fn get_zulu_name(&self) -> Result<&'static str> {
        Ok(match self {
            Architecture::X86 => "x86",
            Architecture::X64 => "x86_64",
            Architecture::ARM => "arm",
            Architecture::AARCH64 => "aarch64",
            _ => bail!("Unsupported architecture for Zulu runtime"),
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
    let max_age = Duration::from_secs(max_age_days * 24 * 60 * 60);

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
