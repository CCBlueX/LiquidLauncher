use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use path_absolutize::Absolutize;
use tokio::fs;
use crate::app::api::ApiEndpoints;

use crate::utils::{download_file, tar_gz_extract, zip_extract};
use crate::utils::{BITNESS, Bitness, OperatingSystem, OS};

/// Find java binary in JRE folder
pub async fn find_java_binary(runtimes_folder: &Path, jre_version: u32) -> Result<PathBuf> {
    let runtime_path = runtimes_folder.join(format!("{}", jre_version));

    // Find JRE in runtime folder
    let mut files = fs::read_dir(&runtime_path).await?;

    if let Some(jre_folder) = files.next_entry().await? {
        let folder_path = jre_folder.path();

        let java_binary = match OS {
            OperatingSystem::WINDOWS => {
                folder_path.join("bin").join("javaw.exe")
            }
            OperatingSystem::OSX => {
                folder_path.join("Contents").join("Home").join("bin").join("java")
            }
            _ => {
                folder_path.join("bin").join("java")
            }
        };

        if java_binary.exists() {
            return Ok(java_binary.absolutize()?.to_path_buf());
        }
    }

    return Err(anyhow::anyhow!("Failed to find JRE"));
}

/// Download specific JRE to runtimes
pub async fn jre_download<F>(runtimes_folder: &Path, jre_version: u32, on_progress: F) -> Result<PathBuf> where F : Fn(u64, u64) {
    let runtime_path = runtimes_folder.join(format!("{}", jre_version));

    if runtime_path.exists() {
        // Clear out folder
        fs::remove_dir_all(&runtime_path).await?;
    }

    fs::create_dir_all(&runtime_path).await?;

    // OS details
    let os_name = match OS {
        OperatingSystem::WINDOWS => "windows",
        OperatingSystem::OSX => "mac",
        OperatingSystem::LINUX => "linux",
        OperatingSystem::UNKNOWN => bail!("Unknown OS")
    }.to_string();

    let os_arch = match BITNESS {
        Bitness::Bit64 => "x64",
        Bitness::Bit32 => "x32",
        Bitness::UNKNOWN => bail!("Unknown bitness")
    }.to_string();

    // Request JRE source
    let jre_source = ApiEndpoints::jre(&os_name, &os_arch, jre_version).await?;

    // Download from JRE source and extract runtime files
    fs::create_dir_all(&runtime_path).await?;

    let retrieved_bytes = download_file(&jre_source.download_url, on_progress).await?;
    let cursor = Cursor::new(&retrieved_bytes[..]);

    match OS {
        OperatingSystem::WINDOWS => zip_extract(cursor, runtime_path.as_path()).await?,
        OperatingSystem::LINUX | OperatingSystem::OSX => tar_gz_extract(cursor, runtime_path.as_path()).await?,
        _ => bail!("Unsupported OS")
    }

    // Find JRE afterwards
    find_java_binary(runtimes_folder, jre_version).await
}

