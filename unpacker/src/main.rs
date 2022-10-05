#![windows_subsystem = "windows"]

use std::{fs, thread};
use std::process::Command;
use rust_embed::RustEmbed;
use anyhow::Result;
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

    // execute assigned executable

    let exit_status = Command::new(temporary_folder.path().join(EXECUTABLE_NAME))
        .current_dir(temporary_folder)
        .status()?;
    assert!(exit_status.success());

    Ok(())
}
