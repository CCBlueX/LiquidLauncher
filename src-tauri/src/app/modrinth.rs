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

pub async fn download_mod(file: &ModrinthFile, dest_path: &std::path::Path) -> Result<()> {
    let response = HTTP_CLIENT
        .get(&file.url)
        .header("User-Agent", "LiquidLauncher/0.5.0")
        .send()
        .await?;

    let bytes = response.bytes().await?;
    tokio::fs::write(dest_path, bytes).await?;

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
