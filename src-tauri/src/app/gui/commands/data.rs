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

use tracing::info;

use crate::{
    app::options::Options,
    LAUNCHER_DIRECTORY
};

#[tauri::command]
pub(crate) async fn get_options() -> Result<Options, String> {
    info!("Loading options...");
    let config_dir = LAUNCHER_DIRECTORY.config_dir();
    let options = Options::load(config_dir).await.unwrap_or_default();
    info!("Done!");
    Ok(options)
}

#[tauri::command]
pub(crate) async fn store_options(options: Options) -> Result<(), String> {
    let config_dir = LAUNCHER_DIRECTORY.config_dir();
    options
        .store(config_dir)
        .await
        .map_err(|e| format!("unable to store config data: {:?}", e))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn clear_data(options: Options) -> Result<(), String> {
    let data_directory = if !options.start_options.custom_data_path.is_empty() {
        Some(options.start_options.custom_data_path)
    } else {
        None
    }
        .map(|x| x.into())
        .unwrap_or_else(|| LAUNCHER_DIRECTORY.data_dir().to_path_buf());

    [
        "assets",
        "gameDir",
        "libraries",
        "mod_cache",
        "natives",
        "runtimes",
        "versions",
    ]
        .iter()
        .map(|dir| data_directory.join(dir))
        .filter(|dir| dir.exists())
        .map(std::fs::remove_dir_all)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("unable to clear data: {:?}", e))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn default_data_folder_path() -> Result<String, String> {
    let data_directory = LAUNCHER_DIRECTORY.data_dir().to_str();

    match data_directory {
        None => Err("unable to get data folder path".to_string()),
        Some(path) => Ok(path.to_string()),
    }
}