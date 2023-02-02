use std::fmt::Display;

use serde::Deserialize;
use sysinfo::{RefreshKind, System, SystemExt};

/// Get the total memory of the system in bytes
pub fn percentage_of_total_memory(memory_percentage: i32) -> i64 {
    let sys = System::new_with_specifics(RefreshKind::new().with_memory());

    ((sys.total_memory() / 1000000) as f64 * (memory_percentage as f64 / 100.0)) as i64
}

pub const OS: Os = if cfg!(windows) {
    Os::WINDOWS
} else if cfg!(unix) {
    Os::LINUX
} else if cfg!(macos) {
    Os::OSX
} else {
    Os::UNKNOWN
};

#[derive(Deserialize, PartialEq, Eq, Hash)]
pub enum Os {
    WINDOWS,
    LINUX,
    OSX,
    UNKNOWN
}

impl Os {
    pub(crate) fn get_path_separator(&self) -> &'static str {
        return match self {
            Os::WINDOWS => ";",
            Os::LINUX | Os::OSX => ":",
            _ => panic!("Invalid OS")
        };
    }
    pub(crate) fn get_simple_name(&self) -> &'static str {
        return match self {
            Os::WINDOWS => "windows",
            Os::LINUX => "linux",
            Os::OSX => "osx",
            _ => panic!("Invalid OS")
        };
    }
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_simple_name())
    }
}