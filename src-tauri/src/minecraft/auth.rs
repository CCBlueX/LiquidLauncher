/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2023 CCBlueX
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
 
use anyhow::{anyhow, Result};
use miners::auth::{self, MsAuth};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use base64::{read::DecoderReader};
use byteorder::{ReadBytesExt, LE};
use uuid::Uuid;
use std::{fs, io::Read, string::String};

use crate::{LAUNCHER_DIRECTORY, HTTP_CLIENT};

/// The client ID of the Azure app used for authentication
pub(crate) const AZURE_CLIENT_ID: &str = "0add8caf-2cc6-4546-b798-c3d171217dd9";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MinecraftAccount {
    #[serde(rename = "Microsoft")]
    MsaAccount {
        name: String,
        uuid: String,
        token: String,
        ms_auth: Option<MsAuth>,
    },
    #[serde(rename = "Offline")]
    OfflineAccount {
        name: String,
        #[serde(deserialize_with = "check_uuid_format")]
        uuid: String
    }
}

fn check_uuid_format<'de, D>(deserializer: D) -> Result<String, D::Error> where D: serde::Deserializer<'de> {
    let uuid = String::deserialize(deserializer)?;

    // If the UUID is invalid, generate a new one
    if uuid.len() < 2 {
        warn!("Invalid UUID: {}, generating random new one.", uuid);
        return Ok(Uuid::new_v4().to_string());
    }

    Ok(uuid)
}

impl MinecraftAccount {

    /// Authenticate using a Microsoft account
    /// Calls `on_code` with the login code, which should be displayed to the user, and then waits for the user to login.
    /// 
    /// WARNING: This will block until the user logs in. If the user does not login, this might block forever.
    /// 
    /// Returns a `MinecraftAccount::MsaAccount` if successful
    pub async fn auth_msa<F>(on_code: F) -> Result<Self> where F: Fn(&String) {
        // Request new device code from Azure
        let device_code = auth::DeviceCode::new(AZURE_CLIENT_ID, None, &HTTP_CLIENT.clone()).await?;

        // Display login code to user
        if let Some(inner) = &device_code.inner { // login code
            on_code(&inner.user_code);
        }
        
        // Authenticate with Azure and wait for the user to login. This will block until the user logs in.
        debug!("Waiting for user to login...");
        let auth = device_code.authenticate(&HTTP_CLIENT.clone()).await?;
        debug!("User logged in!");

        // Return account
        Ok(MinecraftAccount::MsaAccount {
            name: auth.name,
            uuid: auth.uuid,
            token: auth.token,
            ms_auth: Some(auth.ms_auth),
        })
    }

    /// Authenticate using an offline account
    /// Requests the UUID of the username from Mojang's API. If the username is invalid, the UUID will be `-`.
    /// 
    /// Returns a `MinecraftAccount::OfflineAccount` if successful
    pub async fn auth_offline(username: String) -> Self {
        // Request UUID from Mojang's API
        let uuid = uuid_of_username(&username).await
            .unwrap_or_else(|_| Uuid::new_v4().to_string());

        // Return offline account
        MinecraftAccount::OfflineAccount {
            name: username,
            uuid
        }
    }

    /// Refresh access token if necessary
    pub async fn refresh(self) -> Result<MinecraftAccount> {
        return match &self {
            MinecraftAccount::MsaAccount { ms_auth, .. } => {
                let ms_auth = match ms_auth {
                    Some(ms_auth) => ms_auth.clone(),
                    None => read_legacy_ms_auth()?
                };

                let device_code = auth::DeviceCode::new(AZURE_CLIENT_ID, Some(ms_auth), &HTTP_CLIENT.clone()).await?;

                // This is unlikely to happen because we already define a microsoft authentication, but just in case...
                if let Some(_inner) = &device_code.inner { // login code
                    return Err(anyhow!("code required, please re-login!"));
                }

                debug!("Refreshing auth...");

                // Refreshed auth
                let auth = device_code.authenticate(&HTTP_CLIENT.clone()).await?;

                Ok(MinecraftAccount::MsaAccount {
                    name: auth.name,
                    uuid: auth.uuid,
                    token: auth.token,
                    ms_auth: Some(auth.ms_auth),
                })
            }

            MinecraftAccount::OfflineAccount { .. } => Ok(self)
        }
    }

    /// Logout the account
    pub async fn logout(&self) -> Result<()> {
        Ok(())
    }

}

/// Get the UUID of a username
pub async fn uuid_of_username(username: &String) -> Result<String> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    pub enum ApiMojangProfile {
        Success {
            id: String,
            name: String
        },
        Error {
            error: String,
            #[serde(rename(deserialize = "errorMessage"))]
            error_message: String
        }
    }

    // https://api.mojang.com/users/profiles/minecraft/<username>

    let response = HTTP_CLIENT.get(format!("https://api.mojang.com/users/profiles/minecraft/{}", username))
        .send().await?
        .json::<ApiMojangProfile>().await?;

    match response {
        ApiMojangProfile::Success { id, name: _name } => Ok(id),
        ApiMojangProfile::Error { error, error_message } => Err(anyhow!("{}: {}", error, error_message))
    }
}

/// Support for reading legacy authentication files
fn read_legacy_ms_auth() -> Result<MsAuth> {
    let auth_file = LAUNCHER_DIRECTORY.data_dir()
            .join("azure_authentication.cache");

    debug!("Auth file: {:?} (exists: {})", auth_file, auth_file.exists());

    if !auth_file.exists() {
        return Err(anyhow!("auth file does not exist"));
    }

    let msa = read_from(&mut fs::File::open(auth_file)?)?;
    debug!("Read legacy auth: {:?}", msa);
    Ok(msa)
}

fn read_string_from(r: &mut impl Read) -> anyhow::Result<String> {
    let len = r.read_u16::<LE>()?;
    let mut buf = vec![0; len as usize];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf)?)
}

pub fn read_from(r: &mut impl Read) -> anyhow::Result<MsAuth> {
    let mut r = DecoderReader::new(r, base64::STANDARD);
    let len = r.read_u16::<LE>()? as usize;
    let mut buf = vec![0; len];
    r.read_exact(&mut buf)?;
    let mut buf = buf.as_slice();
    let expires_after = buf.read_i64::<LE>()?;
    let access_token = read_string_from(&mut buf)?;
    let refresh_token = read_string_from(&mut buf)?;
    Ok(MsAuth {
        expires_in: 0,
        access_token,
        refresh_token,
        expires_after,
    })
}