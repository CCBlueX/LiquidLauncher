#![feature(once_cell)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
  )]

use std::fs;
use once_cell::sync::Lazy;
use anyhow::Result;
use directories::ProjectDirs;
use log::debug;

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
    debug!("Creating launcher directories...");
    fs::create_dir_all(LAUNCHER_DIRECTORY.data_dir())?;
    fs::create_dir_all(LAUNCHER_DIRECTORY.config_dir())?;

    // app
    app::gui::gui_main();
    return Ok(());
}