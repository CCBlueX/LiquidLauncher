use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AuthenticationError;

// Authentication service
// mojang: https://authserver.mojang.com
// altening: http://authserver.thealtening.com
const MOJANG_AUTH_SERVER: &str = "https://authserver.mojang.com"; 

pub enum AuthService {
    MOJANG
}

impl AuthService {

    // Login with credentials
    pub async fn authenticate(self, username: String, password: String) -> Result<Account> {
        let auth_server = match self {
            AuthService::MOJANG => MOJANG_AUTH_SERVER
        };

        let client = Client::builder().build()?;
        let authenticate_request = client.post(format!("{}/authenticate", auth_server))
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

        #[derive(Deserialize)]
        struct AuthenticateUser {
            // optional
        }

        // Game license
        #[derive(Deserialize)]
        struct AuthenticateProfile {
            name: String,
            id: Uuid
        }
        
        #[derive(Deserialize)]
        struct AuthenticateResponse {
            user: Option<AuthenticateUser>,
            #[serde(rename = "accessToken")]
            access_token: String,
            // #[serde(rename = "availableProfiles")] .. not needed yet
            // available_profiles: ..
            #[serde(rename = "selectedProfile")]
            selected_profile: Option<AuthenticateProfile>
        }

        // todo: handle errors
        // {"error":"ForbiddenOperationException","errorMessage":"Invalid credentials. Invalid username or password."}
        // println!("{}", authenticate_request.text().await?);

        let serialized_response = authenticate_request.json::<AuthenticateResponse>().await?;

        let profile = match serialized_response.selected_profile {
            Some(profile) => profile,
            None => return Err(AuthenticationError::NoGameLicense("Minecraft".into()).into()),
        };

        Ok(Account {
            username: profile.name,
            access_token: serialized_response.access_token,
            id: profile.id,
            account_type: "mojang".to_string()
        })
    }
    
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub username: String,
    #[serde(rename(serialize = "accessToken"))]
    #[serde(alias = "accessToken")]
    pub access_token: String,
    pub id: Uuid,
    #[serde(rename(serialize = "accountType"))]
    #[serde(alias = "accountType")]
    pub account_type: String
}

impl Account {

    // Refresh session
    pub async fn refresh_login(&self) {
        todo!();
    }

    // Logout
    pub async fn invalidate(&self) {
        todo!()
    }

}

