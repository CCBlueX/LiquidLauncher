/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2026 CCBlueX
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

use serde_json::Value;

use super::failed;
use crate::app::translations::{self, Language};

#[tauri::command]
pub(crate) async fn get_languages() -> Vec<Language> {
    translations::languages().await
}

#[tauri::command]
pub(crate) async fn get_translation(code: String) -> Result<Value, String> {
    translations::translation(&code)
        .await
        .map_err(failed("load the translation"))
}
