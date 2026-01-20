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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::HTTP_CLIENT;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Serialize, Deserialize, Clone)]
pub struct ModrinthProject {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub project_id: String,
    pub author: String,
    pub downloads: u64,
}

#[derive(Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<ModrinthProject>,
    pub total_hits: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub files: Vec<ModrinthFile>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
}

pub async fn search_mods(query: &str, mc_version: &str, loader: &str) -> Result<Vec<ModrinthProject>> {
    let facets = format!(
        "[[\"versions:{}\"],[\"categories:{}\"],[\"project_type:mod\"]]",
        mc_version, loader
    );
    
    let url = format!(
        "{}/search?query={}&facets={}&limit=20",
        MODRINTH_API,
        urlencoding::encode(query),
        urlencoding::encode(&facets)
    );

    let response: SearchResponse = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", "LiquidLauncher/0.5.0")
        .send()
        .await?
        .json()
        .await?;

    Ok(response.hits)
}

pub async fn get_compatible_version(project_id: &str, mc_version: &str, loader: &str) -> Result<Option<ModrinthVersion>> {
    let url = format!(
        "{}/project/{}/version?loaders=[\"{}\"]&game_versions=[\"{}\"]",
        MODRINTH_API, project_id, loader, mc_version
    );

    let versions: Vec<ModrinthVersion> = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", "LiquidLauncher/0.5.0")
        .send()
        .await?
        .json()
        .await?;

    Ok(versions.into_iter().next())
}

/// Downloads a mod file with atomic write to prevent corruption on network failure.
/// Uses a temporary file and only moves to final destination after successful download.
pub async fn download_mod(file: &ModrinthFile, dest_path: &std::path::Path) -> Result<()> {
    use anyhow::Context;
    
    let temp_path = dest_path.with_extension("tmp");
    
    let response = HTTP_CLIENT
        .get(&file.url)
        .header("User-Agent", "LiquidLauncher/0.5.0")
        .timeout(std::time::Duration::from_secs(300))
        .send()
        .await
        .context("Network request failed - check your internet connection")?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed with status: {}", response.status());
    }

    let bytes = response.bytes()
        .await
        .context("Download interrupted - connection lost")?;

    // Verify we got the expected size
    if bytes.len() as u64 != file.size {
        anyhow::bail!(
            "Download incomplete: got {} bytes, expected {}",
            bytes.len(),
            file.size
        );
    }

    // Write to temp file first
    tokio::fs::write(&temp_path, &bytes)
        .await
        .context("Failed to write temporary file")?;

    // Remove destination if exists (required for Windows)
    if dest_path.exists() {
        let _ = tokio::fs::remove_file(dest_path).await;
    }

    // Move to final destination
    if let Err(e) = tokio::fs::rename(&temp_path, dest_path).await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(e).context("Failed to save mod file");
    }

    Ok(())
}

pub async fn get_project_from_hash(hash: &str) -> Result<Option<String>> {
    let url = format!("{}/version_file/{}", MODRINTH_API, hash);
    
    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", "LiquidLauncher/0.5.0")
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let version: ModrinthVersion = resp.json().await?;
            Ok(Some(version.project_id))
        }
        _ => Ok(None)
    }
}

/// Get full version info from file hash (SHA-512)
pub async fn get_version_from_hash(hash: &str) -> Result<Option<ModrinthVersion>> {
    let url = format!("{}/version_file/{}", MODRINTH_API, hash);
    
    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", "LiquidLauncher/0.5.0")
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            Ok(Some(resp.json().await?))
        }
        _ => Ok(None)
    }
}

/// Get project details by ID
pub async fn get_project(project_id: &str) -> Result<Option<ModrinthProjectDetails>> {
    let url = format!("{}/project/{}", MODRINTH_API, project_id);
    
    let response = HTTP_CLIENT
        .get(&url)
        .header("User-Agent", "LiquidLauncher/0.5.0")
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            Ok(Some(resp.json().await?))
        }
        _ => Ok(None)
    }
}

#[derive(Deserialize)]
pub struct ModrinthProjectDetails {
    pub id: String,
    pub slug: String,
    pub title: String,
}
