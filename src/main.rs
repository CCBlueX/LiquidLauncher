#![windows_subsystem = "windows"]

#[cfg(feature = "gui")]
#[macro_use]
extern crate sciter;

use std::fs;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use crate::app::option::LauncherOptions;

pub mod app;
pub mod minecraft;

mod error;
mod utils;

const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() -> Result<()> {
    // application directory
    let app_data = match ProjectDirs::from("net", "CCBlueX",  "LiquidLauncher") {
        Some(proj_dirs) => proj_dirs,
        None => return Err(anyhow!("no application directory"))
    };

    fs::create_dir_all(app_data.data_dir())?;
    fs::create_dir_all(app_data.config_dir())?;

    // app

    let args = std::env::args();
    let mut real_args = args.skip(1);

    let mut options = LauncherOptions::load(app_data.config_dir()).unwrap_or_default();
    options.store(app_data.config_dir())?;

    if let Some(build_id) = real_args.next() {
        #[cfg(feature = "cli")]
            {
                let u_build_id = build_id.parse::<u32>().expect("build id not valid");
                app::cli::cli_main(app_data, u_build_id);
                return Ok(());
            }

        eprintln!("This build does not support CLI.");
        return Ok(());
    }

    #[cfg(feature = "gui")]
        {
            app::gui::gui_main(app_data, options);
            return Ok(());
        }

    eprintln!("This build does not support GUI.");
    Ok(())
}