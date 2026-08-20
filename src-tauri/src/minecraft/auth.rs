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

use anyhow::Result;

use minecraft_auth::java::JavaAuthManager;
use minecraft_auth::msa::MsaDeviceCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::gui::ShareableWindow;
use crate::HTTP_CLIENT;

/// Passed to the game process as `--clientId`, an anonymous per-launcher
/// telemetry identifier. Unrelated to `minecraft-auth`'s own Microsoft OAuth
/// application id and unaffected by which login flow was used.
pub(crate) const AZURE_CLIENT_ID: &str = "0add8caf-2cc6-4546-b798-c3d171217dd9";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MinecraftAccount {
    #[serde(rename = "Premium")]
    MsaAccount {
        /// The serialized `JavaAuthManager` (MSA/Xbox Live/Minecraft Services
        /// tokens), refreshed lazily and re-saved after every login/refresh.
        state: serde_json::Value,
        name: String,
        // Named `id`, not `uuid`, to match `OfflineAccount` below — the
        // frontend reads `account.id` regardless of account type.
        id: Uuid,
    },
    #[serde(rename = "Offline")]
    OfflineAccount {
        name: String,
        #[serde(alias = "uuid")]
        id: Uuid,
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

    /// Authenticate using an offline account
    /// Generates UUID from following format: OfflinePlayer:<username>
    /// Java/Kotlin equivalent: UUID.nameUUIDFromBytes("OfflinePlayer:$name".toByteArray())
    ///
    // Explanation: [nameUUIDFromBytes] uses MD5 to generate a UUID from the input bytes.
    // The input bytes are the UTF-8 bytes of the string "OfflinePlayer:$name".
    // The UUID generated is a version 3 UUID, which is based on the MD5 hash of the input bytes.
    ///
    /// Returns a `MinecraftAccount::OfflineAccount` if successful
    pub async fn auth_offline(username: String) -> Self {
        // Generate UUID from "OfflinePlayer:$name"
        let name_str = format!("OfflinePlayer:{}", username);
        let bytes = name_str.as_bytes();
        let mut md5: [u8; 16] = md5::compute(bytes).into();

        md5[6] &= 0x0f; // clear version
        md5[6] |= 0x30; // version 3
        md5[8] &= 0x3f; // clear variant
        md5[8] |= 0x80; // IETF variant

        let uuid = Uuid::from_bytes(md5);

        // Return offline account
        MinecraftAccount::OfflineAccount {
            name: username,
            id: uuid,
        }
    }

    /// Refresh access token if necessary
    pub async fn refresh(self) -> Result<MinecraftAccount> {
        match self {
            MinecraftAccount::MsaAccount { state, .. } => {
                let manager = JavaAuthManager::from_json(HTTP_CLIENT.clone(), &state)?;
                Self::from_manager(manager).await
            }
            offline @ MinecraftAccount::OfflineAccount { .. } => Ok(offline),
        }
    }

    /// Logout the account
    pub async fn logout(&self) -> Result<()> {
        Ok(())
    }

    pub fn get_username(&self) -> &str {
        match self {
            MinecraftAccount::MsaAccount { name, .. } => name,
            MinecraftAccount::OfflineAccount { name, .. } => name,
        }
    }

    /// Drives the manager's token chain to an up-to-date state and captures
    /// it as an account. Shared by fresh logins and `refresh`, so both end
    /// up with a validated Minecraft Services token and current profile.
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
