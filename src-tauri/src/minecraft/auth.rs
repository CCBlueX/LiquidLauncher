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

use azalea_auth::{
    cache::ExpiringValue, get_minecraft_token, get_ms_auth_token, get_ms_link_code, get_profile,
    refresh_ms_auth_token, AccessTokenResponse, AuthError, MinecraftAuthResponse, ProfileResponse,
    XboxLiveAuth,
};
use serde::{Deserialize, Serialize};
use tracing::{error, trace};

use uuid::Uuid;

use crate::HTTP_CLIENT;

/// The client ID of the Azure app used for authentication
pub(crate) const AZURE_CLIENT_ID: &str = "0add8caf-2cc6-4546-b798-c3d171217dd9";
const AZURE_SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MinecraftAccount {
    #[serde(rename = "Premium")]
    MsaAccount {
        /// Microsoft auth
        msa: ExpiringValue<AccessTokenResponse>,
        /// Xbox Live auth
        xbl: ExpiringValue<XboxLiveAuth>,
        /// Minecraft auth
        mca: ExpiringValue<MinecraftAuthResponse>,
        /// The user's Minecraft profile (i.e. username, UUID, skin)
        #[serde(flatten)]
        profile: ProfileResponse,
    },
    #[serde(rename = "Microsoft")]
    LegacyMsaAccount {
        name: String,
        uuid: Uuid,
        token: String,
        ms_auth: MsAuth,
    },
    #[serde(rename = "Offline")]
    OfflineAccount {
        name: String,
        #[serde(alias = "uuid")]
        id: Uuid,
    },
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MsAuth {
    pub expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
    #[serde(skip)]
    pub expires_after: i64,
}

impl MinecraftAccount {
    /// Authenticate using a Microsoft account
    /// Calls `on_code` with the login code, which should be displayed to the user, and then waits for the user to login.
    ///
    /// WARNING: This will block until the user logs in. If the user does not log in, this might block forever.
    ///
    /// Returns a `MinecraftAccount::MsaAccount` if successful
    pub async fn auth_msa<F>(on_code: F) -> Result<Self, AuthError>
    where
        F: Fn(&String, &String),
    {
        // Request new device code from Azure
        let device_code =
            get_ms_link_code(&HTTP_CLIENT, Some(AZURE_CLIENT_ID), Some(AZURE_SCOPE)).await?;
        on_code(&device_code.verification_uri, &device_code.user_code);

        let msa: ExpiringValue<AccessTokenResponse> =
            get_ms_auth_token(&HTTP_CLIENT, device_code, Some(AZURE_CLIENT_ID)).await?;

        login_msa(msa).await
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
            MinecraftAccount::MsaAccount {
                msa,
                xbl,
                mca,
                profile,
                ..
            } => {
                // Not necessary to refresh if the Minecraft auth token is not expired
                if !mca.is_expired() {
                    return Ok(MinecraftAccount::MsaAccount {
                        msa,
                        xbl,
                        mca,
                        profile,
                    });
                }

                // Refresh Microsoft auth token if necessary
                let msa = if msa.is_expired() {
                    trace!("refreshing Microsoft auth token");
                    match refresh_ms_auth_token(
                        &HTTP_CLIENT,
                        &msa.data.refresh_token,
                        Some(AZURE_CLIENT_ID),
                        Some(AZURE_SCOPE),
                    )
                    .await
                    {
                        Ok(new_msa) => new_msa,
                        Err(e) => {
                            // can't refresh, re-authenticate required
                            error!("Error refreshing Microsoft auth token: {}", e);
                            msa
                        }
                    }
                } else {
                    msa
                };

                Ok(login_msa(msa).await?)
            }
            MinecraftAccount::LegacyMsaAccount { ms_auth, .. } => {
                let msa = refresh_ms_auth_token(
                    &HTTP_CLIENT,
                    &ms_auth.refresh_token,
                    Some(AZURE_CLIENT_ID),
                    Some(AZURE_SCOPE),
                )
                .await?;
                Ok(login_msa(msa).await?)
            }
            MinecraftAccount::OfflineAccount { name, id, .. } => {
                Ok(MinecraftAccount::OfflineAccount { name, id })
            }
        }
    }

    /// Logout the account
    pub async fn logout(&self) -> Result<()> {
        Ok(())
    }

    pub fn get_username(&self) -> &str {
        match self {
            MinecraftAccount::MsaAccount { profile, .. } => &profile.name,
            MinecraftAccount::LegacyMsaAccount { name, .. } => name,
            MinecraftAccount::OfflineAccount { name, .. } => name,
        }
    }
}

async fn login_msa(msa: ExpiringValue<AccessTokenResponse>) -> Result<MinecraftAccount, AuthError> {
    let msa_token = &msa.data.access_token;
    trace!("Got access token: {msa_token}");

    let minecraft = get_minecraft_token(&HTTP_CLIENT, msa_token).await?;
    let profile = get_profile(&HTTP_CLIENT, &minecraft.minecraft_access_token).await?;

    // Return account
    Ok(MinecraftAccount::MsaAccount {
        msa,
        xbl: minecraft.xbl,
        mca: minecraft.mca,
        profile,
    })
}
