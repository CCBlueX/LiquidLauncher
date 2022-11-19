#![windows_subsystem = "windows"]

use std::fs;
use std::process::Command;
use rust_embed::RustEmbed;
use anyhow::{Result, anyhow};
use tempfile::tempdir;

/// This is the directory where the files will be bundled from
#[derive(RustEmbed)]
#[folder = "data/"]
struct Data;

/// This is the executable that will be run
const EXECUTABLE_NAME: &str = if cfg!(windows) {
    "liquidlauncher.exe"
} else {
    "liquidlauncher"
};

/// LiquidLauncher Unpacker
/// This program unpacks the LiquidLauncher executable and runs it.
/// It allows us to bundle the requires files with the executable.
fn main() -> Result<()> {
    println!("Unpacking...");

    // Create a temporary directory
    let temporary_folder = tempdir()?;
    println!("Temporary folder: {}", temporary_folder.path().display());

    // Copy the bundles files to the temporary folder
    for file_name in Data::iter() {
        println!("Unpacking {}", file_name);
        if let Some(file) = Data::get(file_name.as_ref()) {
            let path = temporary_folder.path().join(file_name.as_ref());

            // save file
            if let Some(parent) = path.parent() {
                println!("Creating folder {}", parent.display());
                fs::create_dir_all(parent)?;
            }
            print!("Saving file {} ... ", path.display());
            fs::write(path, file.data)?;
            println!("File saved"); // Shows Saving File {} ... File saved.
        }
    }

    // Find the executable path
    let assigned_executable = temporary_folder.path().join(EXECUTABLE_NAME);

    if !assigned_executable.exists() {
        return Err(anyhow!("Executable not found: {}", assigned_executable.display()));
    }

    println!("Assigned executable: {}", assigned_executable.display());

    #[cfg(unix)]
    {
        /// Adjust permissions on Unix Systems
        /// Requires 0o111 permission on the file to be executable
        /// Without it will throw Permission Denied

        use std::os::unix::prelude::PermissionsExt;
        
        println!("Adjusting permissions on UNIX...");

        let metadata = fs::metadata(&assigned_executable)?;
        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        println!("Mode: {}", mode);
        permissions.set_mode(mode | 0o111);
        println!("Setting mode to: {}", permissions.mode());
        fs::set_permissions(&assigned_executable, permissions)?;

        println!("Permissions adjusted");
    }

    // Run the executable

    println!("Executing assigned executable");
    let exit_status = Command::new(assigned_executable)
        .current_dir(&temporary_folder)
        .status();
    println!("Exit status: {:?}", exit_status);

    Ok(())
}
