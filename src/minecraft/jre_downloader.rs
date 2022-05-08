use std::path::PathBuf;

use anyhow::Result;
use os_info::{Bitness, Info};
use path_absolutize::Absolutize;
use serde::Deserialize;
use tokio::fs;

use crate::utils::{download_file, zip_extract};

pub async fn jre_download<F>(jre_version: u32, os_info: &Info, on_progress: F) -> Result<String> where F : Fn(u64, u64) {
    let os_name = os_info.os_type().to_string().to_lowercase();
    let os_arch = match os_info.bitness() {
        Bitness::Unknown => "x32",
        Bitness::X32 => "x32",
        Bitness::X64 => "x64",
        _ => "x32",
    };

    let current_jre_version = reqwest::get(format!("https://api.liquidbounce.net/api/v1/version/jre/{}/{}/{}", os_name, os_arch, jre_version))
        .await?
        .json::<Jre>()
        .await?;

    let mut runtime_path = PathBuf::new();
    runtime_path.push("runtimes");
    runtime_path.push(current_jre_version.version.to_string());

    // Download runtime
    if !runtime_path.exists() {
        fs::create_dir_all(&runtime_path).await?;

        let mut runtime_zip = runtime_path.clone();
        runtime_zip.push("runtime.zip");

        let retrieved_bytes = download_file(&*current_jre_version.download_url, on_progress).await?;
        fs::write(&runtime_zip.as_path(), retrieved_bytes).await?;

        let open_file = fs::File::open(&runtime_zip.as_path()).await?;
        zip_extract(open_file, runtime_path.as_path()).await?;
        fs::remove_file(&runtime_zip).await?;
    }

    // Find JRE
    let mut files = tokio::fs::read_dir(&runtime_path).await?;

    if let Some(jre_folder) = files.next_entry().await? {
        let mut path = jre_folder.path();
        path.push("bin");
        match os_info.os_type() {
            os_info::Type::Windows => path.push("java.exe"),
            os_info::Type::Linux => path.push("java"),
            os_info::Type::Macos => path.push("java"),
            _ => return Err(anyhow::anyhow!("Unsupported OS")),
        }

        return Ok(path.absolutize()?.to_string_lossy().to_string());
    }

    return Err(anyhow::anyhow!("Failed to find JRE"));
}

#[derive(Deserialize)]
pub struct Jre {
    pub version: u32,
    pub download_url: String
}
