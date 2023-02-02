use std::fmt::Display;

use serde::Deserialize;
use sysinfo::{RefreshKind, System, SystemExt};

/// Get the total memory of the system in bytes
pub fn percentage_of_total_memory(memory_percentage: i32) -> i64 {
    let sys = System::new_with_specifics(RefreshKind::new().with_memory());

    ((sys.total_memory() / 1000000) as f64 * (memory_percentage as f64 / 100.0)) as i64
}

pub const OS: OperatingSystem = if cfg!(windows) {
    OperatingSystem::WINDOWS
} else if cfg!(unix) {
    OperatingSystem::LINUX
} else if cfg!(macos) {
    OperatingSystem::OSX
} else {
    OperatingSystem::UNKNOWN
};

pub const BITNESS: Bitness = if cfg!(target_pointer_width = "64") {
    Bitness::Bit64
} else if cfg!(target_pointer_width = "32") {
    Bitness::Bit32
} else {
    Bitness::UNKNOWN
};

pub enum Bitness {
    Bit32,
    Bit64,
    UNKNOWN
}

#[derive(Deserialize, PartialEq, Eq, Hash)]
pub enum OperatingSystem {
    WINDOWS,
    LINUX,
    OSX,
    UNKNOWN
}

impl OperatingSystem {
    pub fn get_path_separator(&self) -> &'static str {
        return match self {
            OperatingSystem::WINDOWS => ";",
            OperatingSystem::LINUX | OperatingSystem::OSX => ":",
            _ => panic!("Invalid OS")
        };
    }
    pub fn get_simple_name(&self) -> &'static str {
        return match self {
            OperatingSystem::WINDOWS => "windows",
            OperatingSystem::LINUX => "linux",
            OperatingSystem::OSX => "osx",
            _ => panic!("Invalid OS")
        };
    }
}

impl Display for OperatingSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_simple_name())
    }
}