use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use oauth2::{
    basic::BasicClient, AccessToken, AuthUrl, AuthorizationCode, ClientId, CsrfToken,
    PkceCodeChallenge, RedirectUrl, RefreshToken, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use tauri::Url;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
use tracing::debug;

use crate::app::client_api::{ApiEndpoints, UserInformation};

const OAUTH_CLIENT_ID: &str = "J2hzqzCxch8hfOPRFNINOZV5Ma4X4BFdZpMjAVEW";
const AUTH_URL: &str = "https://auth.liquidbounce.net/application/o/authorize/";
const TOKEN_URL: &str = "https://auth.liquidbounce.net/application/o/token/";

static SUCCESS_HTML: &str = include_str!("../../static/success.html");

#[derive(Serialize, Deserialize)]
pub struct ClientAccount {
    #[serde(rename = "accessToken")]
    access_token: AccessToken,
    #[serde(rename = "expiresAt")]
    expires_at: u64, // SystemTime
    #[serde(rename = "refreshToken")]
    refresh_token: RefreshToken,
    #[serde(flatten, default)]
    user_information: Option<UserInformation>,
}

impl ClientAccount {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            < SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
    }

    pub fn authenticate_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder> {
        if self.is_expired() {
            bail!("Your client account session has expired! Re-login!");
        }

        Ok(request.bearer_auth(self.access_token.secret()))
    }

    pub fn get_access_token(&self) -> &AccessToken {
        &self.access_token
    }

    pub fn get_refresh_token(&self) -> &RefreshToken {
        &self.refresh_token
    }

    pub fn get_expires_at(&self) -> u64 {
        self.expires_at
    }

    pub async fn update_info(&mut self) -> Result<()> {
        let user_information = ApiEndpoints::user(self).await?;
        self.user_information = Some(user_information.clone());
        Ok(())
    }

    pub fn get_user_information(&self) -> Option<UserInformation> {
        self.user_information.clone()
    }

    pub async fn renew(self) -> Result<ClientAccount> {
        ClientAccountAuthenticator::renew(self.refresh_token).await
    }
}

pub struct ClientAccountAuthenticator;

impl ClientAccountAuthenticator {
    pub async fn start_auth<F>(on_url: F) -> Result<ClientAccount>
    where
        F: Fn(&String),
    {
        // Initialize OAuth client
        let (mut client, http_client) = Self::initialize_oauth().await?;

        // Set up a local redirect server
        let (redirect_uri, listener) = Self::setup_local_redirect().await?;
        client = client.set_redirect_uri(redirect_uri);

        let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate the authorization URL
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_code_challenge)
            .url();

        on_url(&authorize_url.to_string());

        let (code, state) = Self::wait_for_code(listener).await?;

        debug!("OAuth returned the following code:\n{}\n", code.secret());
        debug!(
            "OAuth returned the following state:\n{} (expected `{}`)\n",
            state.secret(),
            csrf_state.secret()
        );

        let token = client
            .exchange_code(code)
            .set_pkce_verifier(pkce_code_verifier)
            .request_async(&http_client)
            .await?;

        debug!("OAuth returned the following token:\n{token:?}\n");
        let expires_at = SystemTime::now() + token.expires_in().context("Missing expires_in")?;

        Ok(ClientAccount {
            access_token: token.access_token().clone(),
            expires_at: expires_at
                .duration_since(UNIX_EPOCH)
                .context("Time went backwards")?
                .as_secs(),
            refresh_token: token
                .refresh_token()
                .context("Missing refresh token")?
                .clone(),
            user_information: None,
        })
    }

    pub async fn renew(refresh_token: RefreshToken) -> Result<ClientAccount> {
        let (client, http_client) = Self::initialize_oauth().await?;

        let token = client
            .exchange_refresh_token(&refresh_token)
            .request_async(&http_client)
            .await?;

        debug!("OAuth returned the following token:\n{token:?}\n");
        let expires_at = SystemTime::now() + token.expires_in().context("Missing expires_in")?;

        Ok(ClientAccount {
            access_token: token.access_token().clone(),
            expires_at: expires_at
                .duration_since(UNIX_EPOCH)
                .context("Time went backwards")?
                .as_secs(),
            refresh_token: token
                .refresh_token()
                .context("Missing refresh token")?
                .clone(),
            user_information: None,
        })
    }

    async fn initialize_oauth() -> Result<(
        oauth2::Client<
            oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
            oauth2::StandardTokenResponse<
                oauth2::EmptyExtraTokenFields,
                oauth2::basic::BasicTokenType,
            >,
            oauth2::StandardTokenIntrospectionResponse<
                oauth2::EmptyExtraTokenFields,
                oauth2::basic::BasicTokenType,
            >,
            oauth2::StandardRevocableToken,
            oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>,
            oauth2::EndpointSet,
            oauth2::EndpointNotSet,
            oauth2::EndpointNotSet,
            oauth2::EndpointNotSet,
            oauth2::EndpointSet,
        >,
        reqwest::Client,
    )> {
        let client_id = ClientId::new(OAUTH_CLIENT_ID.to_string());
        let auth_url =
            AuthUrl::new(AUTH_URL.to_string()).context("Invalid authorization endpoint URL")?;
        let token_url =
            TokenUrl::new(TOKEN_URL.to_string()).context("Invalid token endpoint URL")?;

        let client = BasicClient::new(client_id)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url);

        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("Client should build")?;

        Ok((client, http_client))
    }

    async fn setup_local_redirect() -> Result<(RedirectUrl, TcpListener)> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("Failed to bind to a random port")?;
        let local_addr = listener
            .local_addr()
            .context("Failed to get the local address")?;
        let redirect_uri =
            RedirectUrl::new(format!("http://{}:{}/", local_addr.ip(), local_addr.port()))
                .context("Invalid redirect URL")?;

        Ok((redirect_uri, listener))
    }

    async fn wait_for_code(listener: TcpListener) -> Result<(AuthorizationCode, CsrfToken)> {
        loop {
            if let Ok((mut stream, _)) = listener.accept().await {
                return Self::handle_http_request(&mut stream).await;
            }
        }
    }

    async fn handle_http_request(
        stream: &mut tokio::net::TcpStream,
    ) -> Result<(AuthorizationCode, CsrfToken)> {
        let (reader, mut writer) = stream.split();
        let mut reader = BufReader::new(reader);

        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;

        let redirect_url = request_line.split_whitespace().nth(1).unwrap();
        let url = Url::parse(&("http://127.0.0.1".to_string() + redirect_url))?;

        let code = url
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
            .context("Missing code in the response")?;

        let state = url
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, state)| CsrfToken::new(state.into_owned()))
            .context("Missing state in the response")?;

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-length: {}\r\n\r\n{}",
            SUCCESS_HTML.len(),
            SUCCESS_HTML
        );
        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;

        Ok((code, state))
    }
}
