#[cfg(feature = "gui")]
#[macro_use]
extern crate sciter;

use anyhow::Result;
use env_logger::Env;
use log::*;
use minecraft::{launcher::launch, version::{VersionManifest, VersionProfile}};
use os::OS;
use web_view::Content;

pub mod minecraft;
pub mod cloud;
pub mod os;
mod prelauncher;
#[cfg(feature = "gui")]
mod gui;
#[cfg(feature = "cli")]
mod cli;
mod error;
mod webview_utils;
mod utils;

const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let args = std::env::args();

    let mut real_args = args.skip(1);

    if let Some((mc_version, lb_version)) = real_args.next().zip(real_args.next()) {
        #[cfg(feature = "cli")]
            {
                cli::cli_main(mc_version, lb_version);
                return;
            }

        eprintln!("This build does not support CLI.");
        return;
    }

    #[cfg(feature = "gui")]
        {
            gui::gui_main();
            return;
        }

    eprintln!("This build does not support GUI.");
    return;
}