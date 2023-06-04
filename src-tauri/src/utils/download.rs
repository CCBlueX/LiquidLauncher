use std::path::Path;

use tokio::fs;
use tracing::debug;
use anyhow::Result;

use crate::HTTP_CLIENT;

/// Download file using HTTP_CLIENT without any progress tracking
pub async fn download_file_untracked(url: &str, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref().to_owned();
    let response = HTTP_CLIENT.get(url)
        .send().await?
        .error_for_status()?;
    
    let content = response.bytes().await?;
    fs::write(path, content).await?;
    Ok(())
}

pub async fn download_file<F>(url: &str, on_progress: F) -> Result<Vec<u8>> where F : Fn(u64, u64) {
    debug!("Downloading file {:?}", url);

    let mut response = HTTP_CLIENT.get(url.trim())
        .send().await?
        .error_for_status()?;

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