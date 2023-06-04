
use anyhow::{anyhow, Result};
use miners::auth::{Auth, self};
use reqwest::Client;
use serde_json::json;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::debug;
use uuid::Uuid;

use crate::error::AuthenticationError;
use crate::{LAUNCHER_DIRECTORY, HTTP_CLIENT};

const MOJANG_AUTH_SERVER: &str = "https://authserver.mojang.com";
pub(crate) const AZURE_CLIENT_ID: &str = "0add8caf-2cc6-4546-b798-c3d171217dd9";

pub async fn auth_msa<F>(on_code: F) -> Result<Account>
    where F: Fn(&String) {
    let auth_file = LAUNCHER_DIRECTORY.data_dir()
        .join("azure_authentication.cache");

    debug!("Auth file: {:?} (exists: {})", auth_file, auth_file.exists());
    
    let device_code = auth::DeviceCode::new(AZURE_CLIENT_ID, &auth_file, &HTTP_CLIENT.clone()).await?;

    if let Some(inner) = &device_code.inner { // login code
        on_code(&inner.user_code);
    }
    
    debug!("Authenticating with Azure...");
    let auth = device_code.authenticate(&HTTP_CLIENT.clone()).await?;

    Ok(Account::MsaAccount {
        auth
    })
}

pub async fn auth_offline(username: String) -> Account {
    let uuid = name_to_uuid(&username).await
        .unwrap_or_else(|_| "-".to_string());

    Account::OfflineAccount {
        name: username,
        uuid
    }
}

// Login with credentials
pub async fn authenticate_mojang(username: String, password: String) -> Result<Account> {
    let client = Client::builder().build()?;
    let authenticate_request = client.post(format!("{}/authenticate", MOJANG_AUTH_SERVER))
        .json(&json!({
                "agent": {
                    "name": "Minecraft",
                    "version": 1
                },
                "username": username,
                "password": password,
                "requestUser": false // not required, but maybe in the future
            }))
        .send().await?;

    // Game license
    #[derive(Deserialize)]
    struct AuthenticateProfile {
        name: String,
        id: Uuid
    }

    #[derive(Deserialize)]
    struct AuthenticateResponse {
        #[serde(rename = "accessToken")]
        access_token: String,
        // #[serde(rename = "availableProfiles")] .. not needed yet
        // available_profiles: ..
        #[serde(rename = "selectedProfile")]
        selected_profile: Option<AuthenticateProfile>
    }

    // todo: handle errors
    // {"error":"ForbiddenOperationException","error_message":"Invalid credentials. Invalid username or password."}
    // println!("{}", authenticate_request.text().await?);

    let serialized_response = authenticate_request.json::<AuthenticateResponse>().await?;

    let profile = match serialized_response.selected_profile {
        Some(profile) => profile,
        None => return Err(AuthenticationError::NoGameLicense("Minecraft".into()).into()),
    };

    Ok(Account::MojangAccount {
        name: profile.name,
        token: serialized_response.access_token,
        uuid: profile.id.to_string()
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Account {
    #[serde(rename = "Microsoft")]
    MsaAccount {
        #[serde(flatten)]
        auth: Auth
    },
    #[serde(rename = "Mojang")]
    MojangAccount {
        name: String,
        token: String,
        uuid: String
    },
    #[serde(rename = "Offline")]
    OfflineAccount {
        name: String,
        uuid: String
    }
}

impl Account {

    pub async fn refresh(self) -> Result<Account> {
        return match &self {
            Account::MsaAccount { .. } => {
                let auth_file = LAUNCHER_DIRECTORY.data_dir()
                    .join("azure_authentication.cache");
                let device_code = auth::DeviceCode::new(AZURE_CLIENT_ID, auth_file, &HTTP_CLIENT.clone()).await?;

                if let Some(_inner) = &device_code.inner { // login code
                    return Err(anyhow!("code required, please re-login!"));
                }

                let auth = device_code.authenticate(&HTTP_CLIENT.clone()).await?;

                Ok(Account::MsaAccount {
                    auth
                })
            }
            Account::MojangAccount { .. } => Ok(self),
            Account::OfflineAccount { .. } => Ok(self)
        }
    }

    pub async fn logout(&self) -> Result<()> {
        match self {
            Account::MsaAccount { .. } => {
                let auth_file = LAUNCHER_DIRECTORY.data_dir()
                    .join("azure_authentication.cache");
                fs::remove_file(auth_file).await?;
            }
            Account::MojangAccount { .. } => {}
            Account::OfflineAccount { .. } => {}
        }
        Ok(())
    }

}

pub async fn name_to_uuid(name: &String) -> Result<String> {
    // https://api.mojang.com/users/profiles/minecraft/<username>

    let uuid_response = HTTP_CLIENT.get(format!("https://api.mojang.com/users/profiles/minecraft/{}", name))
        .send().await?
        .json::<UuidResponse>().await?;

    match uuid_response {
        UuidResponse::Success { id, name: _name } => Ok(id),
        UuidResponse::Error { error, error_message } => Err(anyhow!("{}: {}", error, error_message))
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum UuidResponse {
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