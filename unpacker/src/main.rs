#![windows_subsystem = "windows"]

use std::{env, fs, thread};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use rust_embed::RustEmbed;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use tempfile::tempdir;

#[derive(RustEmbed)]
#[folder = "data/"]
struct Data;

const EXECUTABLE_NAME: &str = if cfg!(windows) {
    "liquidlauncher.exe"
} else {
    "liquidlauncher"
};

fn main() -> Result<()> {
    let temporary_folder = tempdir()?;

    println!("temp folder: {:?}", temporary_folder.path());

    for file_name in Data::iter() {
        if let Some(file) = Data::get(file_name.as_ref()) {
            let path = temporary_folder.path().join(file_name.as_ref());

            println!("extracting {}", file_name.as_ref());

            // save file
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, file.data)?;
        }
    }

    // application directory
    let app_data = match ProjectDirs::from("net", "CCBlueX",  "LiquidLauncher") {
        Some(proj_dirs) => proj_dirs,
        None => return Err(anyhow!("no application directory"))
    };

    fs::create_dir_all(app_data.data_dir())?;

    // execute assigned executable

    let exit_status = Command::new(temporary_folder.path().join(EXECUTABLE_NAME))
        .current_dir(app_data.data_dir())
        .status()?;
    assert!(exit_status.success());

    Ok(())
}
