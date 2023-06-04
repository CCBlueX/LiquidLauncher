
use anyhow::{anyhow, Result};
use miners::auth::{Auth, self};

use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::debug;

use crate::{LAUNCHER_DIRECTORY, HTTP_CLIENT};

/// The client ID of the Azure app used for authentication
pub(crate) const AZURE_CLIENT_ID: &str = "0add8caf-2cc6-4546-b798-c3d171217dd9";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MinecraftAccount {
    #[serde(rename = "Microsoft")]
    MsaAccount {
        #[serde(flatten)]
        auth: Auth
    },
    #[serde(rename = "Offline")]
    OfflineAccount {
        name: String,
        uuid: String
    }
}

impl MinecraftAccount {

    /// Authenticate using a Microsoft account
    /// Calls `on_code` with the login code, which should be displayed to the user, and then waits for the user to login.
    /// 
    /// WARNING: This will block until the user logs in. If the user does not login, this might block forever.
    /// 
    /// Returns a `MinecraftAccount::MsaAccount` if successful
    pub async fn auth_msa<F>(on_code: F) -> Result<Self> where F: Fn(&String) {
        // TODO: This should be removed from the auth library and moved to be handled by the launcher itself.
        //       The auth library should only handle the authentication, not the user interaction.
        //       This will allow to store the MS Auth token in the launcher's config file, instead of the auth file.
        //       The launcher should also be able to handle multiple accounts, not just one.
        //       The auth library at the moment also does not support the previous auth file format, which is a problem and needs to be fixed before releasing the launcher.
        let auth_file = LAUNCHER_DIRECTORY.data_dir()
            .join("azure_authentication.cache");

        debug!("Auth file: {:?} (exists: {})", auth_file, auth_file.exists());
        
        // Request new device code from Azure
        let device_code = auth::DeviceCode::new(AZURE_CLIENT_ID, &auth_file, &HTTP_CLIENT.clone()).await?;

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
            auth
            // TODO: Include MS Auth
        })
    }

    /// Authenticate using an offline account
    /// Requests the UUID of the username from Mojang's API. If the username is invalid, the UUID will be `-`.
    /// 
    /// Returns a `MinecraftAccount::OfflineAccount` if successful
    pub async fn auth_offline(username: String) -> Self {
        // Request UUID from Mojang's API
        let uuid = uuid_of_username(&username).await
            .unwrap_or_else(|_| "-".to_string());

        // Return offline account
        MinecraftAccount::OfflineAccount {
            name: username,
            uuid
        }
    }

    /// Refresh access token if necessary
    pub async fn refresh(self) -> Result<MinecraftAccount> {
        return match &self {
            MinecraftAccount::MsaAccount { .. } => {
                let auth_file = LAUNCHER_DIRECTORY.data_dir()
                    .join("azure_authentication.cache");
                let device_code = auth::DeviceCode::new(AZURE_CLIENT_ID, auth_file, &HTTP_CLIENT.clone()).await?;

                if let Some(_inner) = &device_code.inner { // login code
                    return Err(anyhow!("code required, please re-login!"));
                }

                let auth = device_code.authenticate(&HTTP_CLIENT.clone()).await?;

                Ok(MinecraftAccount::MsaAccount {
                    auth
                })
            }
            MinecraftAccount::OfflineAccount { .. } => Ok(self)
        }
    }

    /// Logout the account
    pub async fn logout(&self) -> Result<()> {
        match self {
            MinecraftAccount::MsaAccount { .. } => {
                let auth_file = LAUNCHER_DIRECTORY.data_dir()
                    .join("azure_authentication.cache");
                fs::remove_file(auth_file).await?;
            }
            MinecraftAccount::OfflineAccount { .. } => {}
        }
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

