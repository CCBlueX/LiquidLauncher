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

use std::path::PathBuf;
use serde::Serialize;

#[derive(Serialize)]
pub struct VanillaStatus {
    pub found: bool,
    pub path: String,
    pub saves_count: usize,
    pub resource_packs_count: usize,
    pub shader_packs_count: usize,
}

fn get_vanilla_minecraft_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir().map(|p| p.join(".minecraft"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|p| p.join("Library/Application Support/minecraft"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir().map(|p| p.join(".minecraft"))
    }
}

fn count_entries(path: &PathBuf) -> usize {
    path.read_dir().map(|r| r.count()).unwrap_or(0)
}

#[tauri::command]
pub(crate) async fn get_vanilla_status(custom_path: Option<String>) -> Result<VanillaStatus, String> {
    let mc_dir = if let Some(ref path) = custom_path {
        if !path.is_empty() {
            Some(PathBuf::from(path))
        } else {
            get_vanilla_minecraft_dir()
        }
    } else {
        get_vanilla_minecraft_dir()
    };
    
    match mc_dir {
        Some(path) if path.exists() => {
            Ok(VanillaStatus {
                found: true,
                path: path.to_string_lossy().to_string(),
                saves_count: count_entries(&path.join("saves")),
                resource_packs_count: count_entries(&path.join("resourcepacks")),
                shader_packs_count: count_entries(&path.join("shaderpacks")),
            })
        }
        _ => Ok(VanillaStatus {
            found: false,
            path: String::new(),
            saves_count: 0,
            resource_packs_count: 0,
            shader_packs_count: 0,
        }),
    }
}
