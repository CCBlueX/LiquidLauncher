pub mod os;

use std::path::PathBuf;
use log::debug;
use crate::error::LauncherError;
use anyhow::Result;
use sha1::{Sha1, Digest};

mod extract;

pub use extract::*;

pub fn get_maven_artifact_path(artifact_id: &String) -> Result<String> {
    let split = artifact_id.split(':').collect::<Vec<_>>();

    if split.len() != 3 {
        return Err(LauncherError::InvalidVersionProfile(format!("Invalid artifact name: {}", artifact_id)).into());
    }

    Ok(format!("{}/{name}/{ver}/{name}-{ver}.jar", split[0].replace('.', "/"), name = split[1], ver = split[2]))
}

pub async fn download_file<F>(url: &str, on_progress: F) -> Result<Vec<u8>> where F : Fn(u64, u64) {
    debug!("Downloading file {:?}", url);

    let mut response = reqwest::get(url.trim()).await?.error_for_status()?;

    debug!("Response received from url");

    let max_len = response.content_length().unwrap_or(0);
    let mut output = Vec::with_capacity(max_len as usize);
    let mut curr_len = 0;

    on_progress(0, max_len);

    debug!("Reading data from response chunk...");
    while let Some(data) = response.chunk().await? {
        output.extend_from_slice(&data);
        curr_len += data.len();
        on_progress(curr_len as u64, max_len);
    }

    debug!("Downloaded file");
    Ok(output)
}

pub fn sha1sum(path: &PathBuf) -> Result<String> {
    // get sha1 of library file and check if it matches
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    let hex_hash = base16ct::lower::encode_string(&hash);

    Ok(hex_hash)
}