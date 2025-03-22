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
use crate::app::network::client_api::Client;
use crate::{utils, LAUNCHER_VERSION};
use tracing::{debug, debug_span, info};

#[tauri::command]
pub(crate) async fn get_launcher_version() -> Result<String, String> {
    Ok(LAUNCHER_VERSION.to_string())
}

#[tauri::command]
pub(crate) async fn setup_client() -> Result<Client, String> {
    Client::lookup()
        .await
}

#[tauri::command]
pub(crate) async fn check_system() -> Result<(), String> {
    let span = debug_span!("system_check");
    let _guard = span.enter();

    debug!(parent: &span, "Checking system...");

    #[cfg(windows)]
    {
        use crate::utils::check_hosts_file;
        check_hosts_file().await.map_err(|e| format!("{}", e))?;
    }

    info!(parent: &span, "Successfully checked system.");
    Ok(())
}

#[tauri::command]
pub(crate) fn sys_memory() -> u64 {
    utils::sys_memory() / (1024 * 1024)
}
