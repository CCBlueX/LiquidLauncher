use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use path_absolutize::Absolutize;
use tokio::fs;
use crate::app::api::ApiEndpoints;

use crate::utils::{download_file, tar_extract, zip_extract};
use crate::utils::os::{BITNESS, Bitness, OperatingSystem, OS};

/// Download specific JRE to runtimes
pub async fn jre_download<F>(data: &Path, jre_version: u32, on_progress: F) -> Result<PathBuf> where F : Fn(u64, u64) {
    // runtimes/version_number_of_jre/...
    let runtime_path = data.join("runtimes")
        .join(jre_version.to_string());

    // Download runtime
    if !runtime_path.exists() {
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

        let runtime_archive = runtime_path.join("runtime")
            .with_extension(if OS == OperatingSystem::WINDOWS { "zip" } else { "tar.gz" });

        let retrieved_bytes = download_file(&jre_source.download_url, on_progress).await?;
        fs::write(&runtime_archive.as_path(), retrieved_bytes).await?;

        let open_file = fs::File::open(&runtime_archive.as_path()).await?;
        match OS {
            OperatingSystem::WINDOWS => zip_extract(open_file, runtime_path.as_path()).await?,
            OperatingSystem::LINUX | OperatingSystem::OSX => tar_extract(open_file, runtime_path.as_path()).await?,
            _ => bail!("Unsupported OS")
        }
        fs::remove_file(&runtime_archive).await?;
    }

    // Find JRE in runtime folder
    let mut files = tokio::fs::read_dir(&runtime_path).await?;

    if let Some(jre_folder) = files.next_entry().await? {
        let mut path = jre_folder.path();
        path.push("bin");
        match OS {
            OperatingSystem::WINDOWS => path.push("javaw.exe"),
            _ => path.push("javaw")
        }

        return Ok(path.absolutize()?.to_path_buf());
    }

    return Err(anyhow::anyhow!("Failed to find JRE"));
}

