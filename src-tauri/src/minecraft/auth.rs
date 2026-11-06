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

use anyhow::{bail, Result};

use minecraft_auth::java::{JavaAuthManager, MinecraftProfile};
use minecraft_auth::msa::MsaDeviceCode;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::gui::ShareableWindow;
use crate::HTTP_CLIENT;

const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MinecraftAccount {
    #[serde(rename = "Premium")]
    MsaAccount {
        /// The serialized `JavaAuthManager` (MSA/Xbox Live/Minecraft Services
        /// tokens), refreshed lazily and re-saved after every login/refresh.
        state: serde_json::Value,
        name: String,
        id: Uuid,
    },
    #[serde(rename = "Offline")]
    OfflineAccount {
        name: String,
        #[serde(alias = "uuid")]
        id: Uuid,
    },
    /// A Minecraft access token without a Microsoft sign-in behind it.
    #[serde(rename = "Session")]
    SessionAccount {
        name: String,
        id: Uuid,
        token: String,
    },
}

impl MinecraftAccount {
    /// Authenticate using a Microsoft account via the device code flow.
    /// `on_code` is called once the code has been requested, and should
    /// display it and its verification URL to the user.
    ///
    /// WARNING: This will block until the user logs in or the code expires.
    pub async fn auth_msa_device_code<F>(on_code: F) -> Result<Self>
    where
        F: FnOnce(&MsaDeviceCode),
    {
        let manager = JavaAuthManager::builder(HTTP_CLIENT.clone())
            .login_device_code(on_code)
            .await?;

        Self::from_manager(manager).await
    }

    /// Authenticate using a Microsoft account via an embedded sign-in
    /// window. Opens a child webview on `window` pointed at Microsoft's
    /// login page and waits for it to complete.
    pub async fn auth_msa_webview(window: ShareableWindow) -> Result<Self> {
        let manager = JavaAuthManager::builder(HTTP_CLIENT.clone())
            .login_webview(move |authorize_url| {
                let window = window.clone();
                async move {
                    crate::app::webview::show_msa_login_webview(&window, authorize_url)
                        .await
                        .map_err(|e| minecraft_auth::Error::Webview(e.to_string()))
                }
            })
            .await?;

        Self::from_manager(manager).await
    }

    pub fn auth_offline(username: String) -> Self {
        MinecraftAccount::OfflineAccount {
            id: offline_uuid(&username),
            name: username,
        }
    }

    pub async fn auth_msa_refresh_token(refresh_token: &str) -> Result<Self> {
        let manager = JavaAuthManager::builder(HTTP_CLIENT.clone())
            .login_refresh_token(refresh_token)
            .await?;

        Self::from_manager(manager).await
    }

    pub async fn auth_session(token: String) -> Result<Self> {
        let response = HTTP_CLIENT
            .get(PROFILE_URL)
            .bearer_auth(&token)
            .send()
            .await?;

        match response.status() {
            StatusCode::NOT_FOUND => return Err(minecraft_auth::Error::ProfileNotFound.into()),
            StatusCode::UNAUTHORIZED => bail!("This token is invalid or expired."),
            _ => {}
        }

        let profile: MinecraftProfile = response.error_for_status()?.json().await?;
        Ok(MinecraftAccount::SessionAccount {
            name: profile.name,
            id: profile.id,
            token,
        })
    }

    /// Refresh the access token if necessary
    pub async fn refresh(self) -> Result<MinecraftAccount> {
        match self {
            MinecraftAccount::MsaAccount { state, .. } => {
                let manager = JavaAuthManager::from_json(HTTP_CLIENT.clone(), &state)?;
                Self::from_manager(manager).await
            }
            offline @ MinecraftAccount::OfflineAccount { .. } => Ok(offline),
            session @ MinecraftAccount::SessionAccount { .. } => Ok(session),
        }
    }

    pub fn get_username(&self) -> &str {
        match self {
            MinecraftAccount::MsaAccount { name, .. } => name,
            MinecraftAccount::OfflineAccount { name, .. } => name,
            MinecraftAccount::SessionAccount { name, .. } => name,
        }
    }

    async fn from_manager(manager: JavaAuthManager) -> Result<Self> {
        manager.minecraft_token().await?;
        let profile = manager.profile().await?;
        let state = manager.to_json().await?;

        Ok(MinecraftAccount::MsaAccount {
            state,
            name: profile.name,
            id: profile.id,
        })
    }
}

/// Same as Java's `UUID.nameUUIDFromBytes("OfflinePlayer:<name>")`.
pub fn offline_uuid(username: &str) -> Uuid {
    let name_str = format!("OfflinePlayer:{}", username);
    let mut md5: [u8; 16] = md5::compute(name_str.as_bytes()).into();

    md5[6] &= 0x0f; // clear version
    md5[6] |= 0x30; // version 3
    md5[8] &= 0x3f; // clear variant
    md5[8] |= 0x80; // IETF variant

    Uuid::from_bytes(md5)
}
