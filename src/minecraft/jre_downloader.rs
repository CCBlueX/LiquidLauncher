use std::path::{Path, PathBuf};

use anyhow::Result;
use os_info::{Bitness, Info, Type};
use path_absolutize::Absolutize;
use serde::Deserialize;
use tokio::fs;

use crate::utils::{download_file, zip_extract};

/// Download specific JRE to runtimes
pub async fn jre_download<F>(data: &Path, jre_version: u32, os_info: &Info, on_progress: F) -> Result<PathBuf> where F : Fn(u64, u64) {
    // runtimes/version_number_of_jre/...
    let runtime_path = data.join("runtimes")
        .join(jre_version.to_string());

    // Download runtime
    if !runtime_path.exists() {
        // OS details
        let os_name = match os_info.os_type() {
            Type::Macos => "mac",
            Type::Windows => "windows",
            _ => "linux",
        };
        let os_arch = match os_info.bitness() {
            Bitness::X64 => "x64",
            _ => "x32",
        };

        // Request JRE source
        let jre_source = reqwest::get(format!("https://api.liquidbounce.net/api/v1/version/jre/{}/{}/{}", os_name, os_arch, jre_version))
            .await?
            .json::<JreSource>()
            .await?;

        // Download from JRE source and extract runtime files
        fs::create_dir_all(&runtime_path).await?;

        let mut runtime_zip = runtime_path.clone();
        runtime_zip.push("runtime.zip");

        let retrieved_bytes = download_file(&*jre_source.download_url, on_progress).await?;
        fs::write(&runtime_zip.as_path(), retrieved_bytes).await?;

        let open_file = fs::File::open(&runtime_zip.as_path()).await?;
        zip_extract(open_file, runtime_path.as_path()).await?;
        fs::remove_file(&runtime_zip).await?;
    }

    // Find JRE in runtime folder
    let mut files = tokio::fs::read_dir(&runtime_path).await?;

    if let Some(jre_folder) = files.next_entry().await? {
        let mut path = jre_folder.path();
        path.push("bin");
        match os_info.os_type() {
            Type::Windows => path.push("javaw.exe"),
            _ => path.push("javaw")
        }

        return Ok(path.absolutize()?.to_path_buf());
    }

    return Err(anyhow::anyhow!("Failed to find JRE"));
}

#[derive(Deserialize)]
pub struct JreSource {
    pub version: u32,
    pub download_url: String
}
