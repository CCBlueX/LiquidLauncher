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

use tracing::warn;

use super::data_directory;
use crate::{
    app::options::Options,
    auth::shared,
    minecraft::game_dir::GameDir,
    LAUNCHER_DIRECTORY
};

#[tauri::command]
pub(crate) async fn get_options() -> Result<Options, String> {
    let config_dir = LAUNCHER_DIRECTORY.config_dir();
    let mut options = Options::load(config_dir).await.unwrap_or_default();
    let data = data_directory(&options).await?;

    if let Some(account) = options.premium_options.account.take() {
        let moved = async {
            if shared::read(&data).await?.is_none() {
                shared::write(&data, Some(&account)).await?;
            }
            options.store(config_dir).await
        };
        if let Err(error) = moved.await {
            warn!("Failed to move the client account to LiquidBounce: {:?}", error);
        }
    }

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
    let data = data_directory(&options).await?;

    [
        "assets",
        "libraries",
        "mod_cache",
        "natives",
        "runtimes",
        "versions",
    ]
        .iter()
        .map(|dir| data.join(dir))
        .chain([GameDir::root(&data)])
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