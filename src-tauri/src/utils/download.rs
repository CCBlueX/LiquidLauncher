/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2024 CCBlueX
 *
 * LiquidLauncher is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * LiquidLauncher is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with LiquidLauncher. If not, see <https://www.gnu.org/licenses/>.
 */

use std::path::Path;

use anyhow::Result;
use tokio::fs;
use tracing::debug;

use crate::HTTP_CLIENT;

/// Download file using HTTP_CLIENT without any progress tracking
pub async fn download_file_untracked(url: &str, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref().to_owned();
    let response = HTTP_CLIENT.get(url).send().await?.error_for_status()?;

    let content = response.bytes().await?;
    fs::write(path, content).await?;
    Ok(())
}

pub async fn download_file<F>(url: &str, on_progress: F) -> Result<Vec<u8>>
where
    F: Fn(u64, u64),
{
    debug!("Downloading file {:?}", url);

    let mut response = HTTP_CLIENT
        .get(url.trim())
        .send()
        .await?
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
