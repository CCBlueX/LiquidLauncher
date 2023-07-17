use std::fmt::Display;
use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;
use sysinfo::{RefreshKind, System, SystemExt};

/// Get the total memory of the system in bytes
pub fn percentage_of_total_memory(memory_percentage: i32) -> i64 {
    let sys = System::new_with_specifics(RefreshKind::new().with_memory());

    ((sys.total_memory() / 1000000) as f64 * (memory_percentage as f64 / 100.0)) as i64
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

pub const OS_VERSION: Lazy<String> = Lazy::new(|| {
    os_info::get().version().to_string()
});

#[derive(Deserialize, PartialEq, Eq, Hash, Debug)]
pub enum OperatingSystem {
    #[serde(rename = "windows")]
    WINDOWS,
    #[serde(rename = "linux")]
    LINUX,
    #[serde(rename = "osx")]
    OSX,
    #[serde(rename = "unknown")]
    UNKNOWN
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
    UNKNOWN
}

impl OperatingSystem {
    pub fn get_path_separator(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => ";",
            OperatingSystem::LINUX | OperatingSystem::OSX => ":",
            _ => bail!("Invalid OS")
        })
    }

    pub fn get_simple_name(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => "windows",
            OperatingSystem::LINUX => "linux",
            OperatingSystem::OSX => "osx",
            _ => bail!("Invalid OS")
        })
    }

    pub fn get_adoptium_name(&self) -> Result<&'static str> {
        Ok(match self {
            OperatingSystem::WINDOWS => "windows",
            OperatingSystem::LINUX => "linux",
            OperatingSystem::OSX => "mac",
            _ => bail!("Invalid OS")
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
            _ => bail!("Invalid architecture")
        })
    }
}

impl Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_simple_name().unwrap())
    }
}