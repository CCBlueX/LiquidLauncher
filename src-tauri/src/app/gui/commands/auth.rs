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

use std::sync::{Arc, Mutex};

use tauri::{Emitter, Window};
use tracing::{debug, error, info};

use crate::app::client_api::{Client, UserInformation};
use crate::app::gui::AppState;
use crate::app::options::Options;
use crate::{
    auth::{shared, ClientAccountAuthenticator},
    minecraft::auth::MinecraftAccount,
};

#[tauri::command]
pub(crate) async fn login_offline(username: &str) -> Result<MinecraftAccount, String> {
    let account = MinecraftAccount::auth_offline(username.to_string()).await;
    Ok(account)
}

#[tauri::command]
pub(crate) async fn login_microsoft_device_code(window: Window) -> Result<MinecraftAccount, String> {
    let account = MinecraftAccount::auth_msa_device_code(|device_code| {
        debug!(
            "enter code {} at {} to sign-in",
            device_code.user_code, device_code.verification_uri
        );
        let _ = window.emit(
            "microsoft_device_code",
            serde_json::json!({
                "userCode": device_code.user_code,
                "verificationUri": device_code.verification_uri,
                "directVerificationUri": device_code.direct_verification_uri(),
            }),
        );
    })
        .await
        .map_err(|e| format!("{}", e))?;

    Ok(account)
}

#[tauri::command]
pub(crate) async fn login_microsoft_webview(window: Window) -> Result<MinecraftAccount, String> {
    let account = MinecraftAccount::auth_msa_webview(Arc::new(Mutex::new(window)))
        .await
        .map_err(|e| format!("{}", e))?;

    Ok(account)
}

#[tauri::command]
pub(crate) async fn client_account(
    client: Client,
    options: Options,
) -> Result<Option<UserInformation>, String> {
    let data = options.start_options.data_directory();
    let Some(mut account) = shared::read(&data)
        .await
        .map_err(|e| format!("unable to read client account: {:?}", e))?
    else {
        return Ok(None);
    };

    if account.is_expired() {
        account = shared::renew(&data, account)
            .await
            .map_err(|e| format!("unable to update access token: {:?}", e))?;
    }

    client
        .fetch_user(&account)
        .await
        .map(Some)
        .map_err(|e| format!("unable to fetch user information: {:?}", e))
}

#[tauri::command]
pub(crate) async fn client_account_authenticate(
    client: Client,
    options: Options,
    app_state: tauri::State<'_, AppState>,
) -> Result<UserInformation, String> {
    ensure_client_stopped(&app_state)?;

    let account = ClientAccountAuthenticator::start_auth(|uri| {
        let _ = tauri_plugin_opener::open_url(uri, None::<&str>);
    })
        .await
        .map_err(|e| format!("{}", e))?;

    let user = client
        .fetch_user(&account)
        .await
        .map_err(|e| format!("unable to fetch user information: {:?}", e))?;

    shared::write(&options.start_options.data_directory(), Some(&account))
        .await
        .map_err(|e| format!("unable to store client account: {:?}", e))?;
    Ok(user)
}

#[tauri::command]
pub(crate) async fn client_account_logout(
    options: Options,
    app_state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    ensure_client_stopped(&app_state)?;

    shared::write(&options.start_options.data_directory(), None)
        .await
        .map_err(|e| format!("unable to store client account: {:?}", e))
}

fn ensure_client_stopped(app_state: &AppState) -> Result<(), String> {
    let running = app_state
        .runner_instance
        .lock()
        .map_err(|e| format!("unable to lock runner instance: {:?}", e))?
        .is_some();

    if running {
        return Err("close LiquidBounce first, it would overwrite the account when it exits".to_string());
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn refresh(account_data: MinecraftAccount) -> Result<MinecraftAccount, String> {
    info!("Refreshing account...");
    let account = account_data.refresh().await.map_err(|e| {
        // The frontend only logs this to the webview console, which never
        // reaches launcher.log.
        error!("Failed to refresh account: {:?}", e);
        format!("unable to refresh: {:?}", e)
    })?;
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