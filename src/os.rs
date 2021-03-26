use std::fmt::Display;

use serde::Deserialize;

pub const OS: Os = if cfg!(windows) {
    Os::WINDOWS
}else if cfg!(unix) {
    Os::LINUX
}else if cfg!(macos) {
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

impl Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Os::WINDOWS => f.write_str("windows"),
            Os::LINUX => f.write_str("linux"),
            Os::OSX => f.write_str("osx"),
            Os::UNKNOWN => f.write_str("unexpected os"),
        }
    }
}