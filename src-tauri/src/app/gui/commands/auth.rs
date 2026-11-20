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
    auth::{
        accounts::{self, AccountEntry},
        shared, ClientAccountAuthenticator,
    },
    minecraft::auth::MinecraftAccount,
};

/// Set to `1`, offline accounts no longer need a Microsoft account in the list.
const ALLOW_OFFLINE_ENV: &str = "LIQUIDLAUNCHER_ALLOW_OFFLINE";

/// The frontend shows "Get Minecraft to play" for this error.
const NO_MINECRAFT: &str = "no_minecraft";

#[tauri::command]
pub(crate) async fn list_accounts(options: Options) -> Result<Vec<AccountEntry>, String> {
    accounts::list(&options.start_options.data_directory())
        .await
        .map_err(error_text)
}

#[tauri::command]
pub(crate) async fn select_account(
    options: Options,
    id: String,
) -> Result<MinecraftAccount, String> {
    accounts::select(&options.start_options.data_directory(), &id)
        .await
        .map_err(error_text)
}

#[tauri::command]
pub(crate) async fn add_microsoft_webview(
    window: Window,
    options: Options,
    app_state: tauri::State<'_, AppState>,
) -> Result<MinecraftAccount, String> {
    ensure_client_stopped(&app_state)?;

    let account = MinecraftAccount::auth_msa_webview(Arc::new(Mutex::new(window)))
        .await
        .map_err(error_text)?;
    save(&app_state, &options, account).await
}

#[tauri::command]
pub(crate) async fn add_microsoft_device_code(
    window: Window,
    options: Options,
    app_state: tauri::State<'_, AppState>,
) -> Result<MinecraftAccount, String> {
    ensure_client_stopped(&app_state)?;

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
    .map_err(error_text)?;
    save(&app_state, &options, account).await
}

#[tauri::command]
pub(crate) async fn add_offline(
    options: Options,
    username: String,
    app_state: tauri::State<'_, AppState>,
) -> Result<MinecraftAccount, String> {
    let username = username.trim();
    if username.is_empty() {
        return Err("Enter a username.".to_string());
    }
    require_microsoft(&options).await?;

    let account = MinecraftAccount::auth_offline(username.to_string());
    save(&app_state, &options, account).await
}

#[tauri::command]
pub(crate) async fn add_session(
    options: Options,
    token: String,
    app_state: tauri::State<'_, AppState>,
) -> Result<MinecraftAccount, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Enter a token.".to_string());
    }

    // LiquidBounce's session login takes a Microsoft refresh token the same way.
    let account = if token.starts_with("M.") {
        MinecraftAccount::auth_msa_refresh_token(token).await
    } else {
        require_microsoft(&options).await?;
        MinecraftAccount::auth_session(token.to_string()).await
    }
    .map_err(error_text)?;
    save(&app_state, &options, account).await
}

#[tauri::command]
pub(crate) async fn remove_account(
    options: Options,
    id: String,
    app_state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    ensure_client_stopped(&app_state)?;

    accounts::remove(&options.start_options.data_directory(), &id)
        .await
        .map_err(error_text)
}

#[tauri::command]
pub(crate) async fn set_favorite(
    options: Options,
    id: String,
    favorite: bool,
    app_state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    ensure_client_stopped(&app_state)?;

    accounts::set_favorite(&options.start_options.data_directory(), &id, favorite)
        .await
        .map_err(error_text)
}

#[tauri::command]
pub(crate) fn offline_login_available() -> bool {
    std::env::var(ALLOW_OFFLINE_ENV).is_ok_and(|value| value == "1")
}

async fn require_microsoft(options: &Options) -> Result<(), String> {
    if offline_login_available() {
        return Ok(());
    }

    match accounts::has_microsoft(&options.start_options.data_directory()).await {
        Ok(true) => Ok(()),
        Ok(false) => Err("Add a Microsoft account first.".to_string()),
        Err(error) => Err(error_text(error)),
    }
}

async fn save(
    app_state: &AppState,
    options: &Options,
    account: MinecraftAccount,
) -> Result<MinecraftAccount, String> {
    // A sign-in takes a while, the client may have been started since.
    ensure_client_stopped(app_state)?;

    accounts::add(&options.start_options.data_directory(), &account)
        .await
        .map_err(error_text)?;
    Ok(account)
}

fn error_text(error: anyhow::Error) -> String {
    match error.downcast_ref::<minecraft_auth::Error>() {
        Some(minecraft_auth::Error::ProfileNotFound) => NO_MINECRAFT.to_string(),
        _ => format!("{:#}", error),
    }
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