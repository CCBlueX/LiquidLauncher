#![feature(once_cell)]
#![windows_subsystem = "windows"]

#[cfg(feature = "gui")]
#[macro_use]
extern crate sciter;

use std::fs;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use minceraft::auth;
use crate::app::app_data::LauncherOptions;

pub mod app;
pub mod minecraft;

mod error;
mod utils;

pub mod updater;

const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
static LAUNCHER_DIRECTORY: Lazy<ProjectDirs> = Lazy::new(|| {
    match ProjectDirs::from("net", "CCBlueX",  "LiquidLauncher") {
        Some(proj_dirs) => proj_dirs,
        None => panic!("no application directory")
    }
});

pub fn main() -> Result<()> {
    // application directory
    fs::create_dir_all(LAUNCHER_DIRECTORY.data_dir())?;
    fs::create_dir_all(LAUNCHER_DIRECTORY.config_dir())?;

    // app

    let args = std::env::args();
    let mut real_args = args.skip(1);

    if let Some(build_id) = real_args.next() {
        #[cfg(feature = "cli")]
            {
                let u_build_id = build_id.parse::<u32>().expect("build id not valid");
                app::cli::cli_main(u_build_id);
                return Ok(());
            }

        eprintln!("This build does not support CLI.");
        return Ok(());
    }

    #[cfg(feature = "gui")]
        {
            app::gui::gui_main();
            return Ok(());
        }

    eprintln!("This build does not support GUI.");
    Ok(())
}