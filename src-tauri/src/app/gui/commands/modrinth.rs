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

use crate::app::modrinth::{self, ModrinthProject, ModrinthVersion};
use crate::LAUNCHER_DIRECTORY;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledModInfo {
    pub project_id: String,
    pub version_id: String,
    pub filename: String,
    pub title: String,
}

#[derive(Serialize)]
pub struct ModWithUpdate {
    pub info: InstalledModInfo,
    pub has_update: bool,
    pub new_version: Option<String>,
}

async fn get_metadata_path(branch: &str, mc_version: &str) -> PathBuf {
    let data = LAUNCHER_DIRECTORY.data_dir();
    data.join("custom_mods")
        .join(format!("{}-{}", branch, mc_version))
        .join(".modrinth_meta.json")
}

async fn load_metadata(branch: &str, mc_version: &str) -> HashMap<String, InstalledModInfo> {
    let path = get_metadata_path(branch, mc_version).await;
    if let Ok(content) = fs::read_to_string(&path).await {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

async fn save_metadata(branch: &str, mc_version: &str, metadata: &HashMap<String, InstalledModInfo>) {
    let path = get_metadata_path(branch, mc_version).await;
    if let Ok(json) = serde_json::to_string(metadata) {
        let _ = fs::write(&path, json).await;
    }
}

#[tauri::command]
pub(crate) async fn modrinth_search(
    query: String,
    mc_version: String,
    loader: String,
) -> Result<Vec<ModrinthProject>, String> {
    modrinth::search_mods(&query, &mc_version, &loader)
        .await
        .map_err(|e| format!("Search failed: {:?}", e))
}

#[tauri::command]
pub(crate) async fn modrinth_get_version(
    project_id: String,
    mc_version: String,
    loader: String,
) -> Result<Option<ModrinthVersion>, String> {
    modrinth::get_compatible_version(&project_id, &mc_version, &loader)
        .await
        .map_err(|e| format!("Failed to get version: {:?}", e))
}

#[tauri::command]
pub(crate) async fn modrinth_install(
    project_id: String,
    mc_version: String,
    loader: String,
    branch: String,
    title: String,
) -> Result<String, String> {
    let version = modrinth::get_compatible_version(&project_id, &mc_version, &loader)
        .await
        .map_err(|e| format!("Failed to get version: {:?}", e))?
        .ok_or("No compatible version found")?;

    let file = version.files.iter()
        .find(|f| f.primary)
        .or(version.files.first())
        .ok_or("No files available")?;

    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_path = data
        .join("custom_mods")
        .join(format!("{}-{}", branch, mc_version));

    if !mod_path.exists() {
        fs::create_dir_all(&mod_path).await
            .map_err(|e| format!("Failed to create directory: {:?}", e))?;
    }

    let dest = mod_path.join(&file.filename);
    
    modrinth::download_mod(file, &dest)
        .await
        .map_err(|e| format!("Download failed: {:?}", e))?;

    // Save metadata
    let mut metadata = load_metadata(&branch, &mc_version).await;
    metadata.insert(project_id.clone(), InstalledModInfo {
        project_id,
        version_id: version.id,
        filename: file.filename.clone(),
        title,
    });
    save_metadata(&branch, &mc_version, &metadata).await;

    Ok(file.filename.clone())
}

#[tauri::command]
pub(crate) async fn modrinth_get_installed(
    branch: String,
    mc_version: String,
) -> Result<Vec<InstalledModInfo>, String> {
    let metadata = load_metadata(&branch, &mc_version).await;
    Ok(metadata.into_values().collect())
}

#[tauri::command]
pub(crate) async fn modrinth_check_updates(
    branch: String,
    mc_version: String,
    loader: String,
) -> Result<Vec<ModWithUpdate>, String> {
    let metadata = load_metadata(&branch, &mc_version).await;
    let mut results = Vec::new();

    for info in metadata.values() {
        let latest = modrinth::get_compatible_version(&info.project_id, &mc_version, &loader)
            .await
            .ok()
            .flatten();

        let (has_update, new_version) = match latest {
            Some(v) if v.id != info.version_id => (true, Some(v.version_number)),
            _ => (false, None),
        };

        results.push(ModWithUpdate {
            info: info.clone(),
            has_update,
            new_version,
        });
    }

    Ok(results)
}

#[tauri::command]
pub(crate) async fn modrinth_update_mod(
    project_id: String,
    mc_version: String,
    loader: String,
    branch: String,
) -> Result<String, String> {
    let mut metadata = load_metadata(&branch, &mc_version).await;
    let old_info = metadata.get(&project_id).cloned();

    let version = modrinth::get_compatible_version(&project_id, &mc_version, &loader)
        .await
        .map_err(|e| format!("Failed to get version: {:?}", e))?
        .ok_or("No compatible version found")?;

    let file = version.files.iter()
        .find(|f| f.primary)
        .or(version.files.first())
        .ok_or("No files available")?;

    let data = LAUNCHER_DIRECTORY.data_dir();
    let mod_path = data
        .join("custom_mods")
        .join(format!("{}-{}", branch, mc_version));

    // Delete old file
    if let Some(old) = &old_info {
        let old_path = mod_path.join(&old.filename);
        if old_path.exists() {
            fs::remove_file(&old_path).await.ok();
        }
    }

    // Download new file
    let dest = mod_path.join(&file.filename);
    modrinth::download_mod(file, &dest)
        .await
        .map_err(|e| format!("Download failed: {:?}", e))?;

    // Update metadata
    if let Some(old) = old_info {
        metadata.insert(project_id.clone(), InstalledModInfo {
            project_id,
            version_id: version.id,
            filename: file.filename.clone(),
            title: old.title,
        });
        save_metadata(&branch, &mc_version, &metadata).await;
    }

    Ok(file.filename.clone())
}
