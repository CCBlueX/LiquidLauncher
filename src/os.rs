use std::fmt::Display;

use serde::Deserialize;

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
}

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match self {
                Os::WINDOWS => "windows",
                Os::LINUX => "linux",
                Os::OSX => "osx",
                _ => panic!("Invalid OS")
            }
        )
    }
}