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

use tauri::{Emitter, Window};
use tracing::{debug, info};

use crate::{
    auth::{ClientAccount, ClientAccountAuthenticator},
    minecraft::auth::MinecraftAccount,
};

#[tauri::command]
pub(crate) async fn login_offline(username: &str) -> Result<MinecraftAccount, String> {
    let account = MinecraftAccount::auth_offline(username.to_string()).await;
    Ok(account)
}

#[tauri::command]
pub(crate) async fn login_microsoft(window: Window) -> Result<MinecraftAccount, String> {
    let account = MinecraftAccount::auth_msa(|uri, code| {
        debug!("enter code {} on {} to sign-in", code, uri);
        let _ = window.emit("microsoft_code", code);
    })
        .await
        .map_err(|e| format!("{}", e))?;

    Ok(account)
}

#[tauri::command]
pub(crate) async fn client_account_authenticate() -> Result<ClientAccount, String> {
    let mut account = ClientAccountAuthenticator::start_auth(|uri| {
        let _ = tauri_plugin_opener::open_url(uri, None::<&str>);
    })
        .await
        .map_err(|e| format!("{}", e))?;

    account
        .update_info()
        .await
        .map_err(|e| format!("unable to fetch user information: {:?}", e))?;

    Ok(account)
}

#[tauri::command]
pub(crate) async fn client_account_update(account: ClientAccount) -> Result<ClientAccount, String> {
    let mut account = account
        .renew()
        .await
        .map_err(|e| format!("unable to update access token: {:?}", e))?;

    account
        .update_info()
        .await
        .map_err(|e| format!("unable to fetch user information: {:?}", e))?;
    Ok(account)
}

#[tauri::command]
pub(crate) async fn refresh(account_data: MinecraftAccount) -> Result<MinecraftAccount, String> {
    info!("Refreshing account...");
    let account = account_data
        .refresh()
        .await
        .map_err(|e| format!("unable to refresh: {:?}", e))?;
    info!(
        "Account was refreshed - username {}",
        account.get_username()
    );
    Ok(account)
}

#[tauri::command]
pub(crate) async fn logout(account_data: MinecraftAccount) -> Result<(), String> {
    account_data
        .logout()
        .await
        .map_err(|e| format!("unable to logout: {:?}", e))
}