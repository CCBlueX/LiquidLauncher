use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use oauth2::{basic::BasicClient, AccessToken, AuthUrl, AuthorizationCode, ClientId, CsrfToken, RedirectUrl, RefreshToken, StandardTokenResponse, TokenResponse, TokenUrl};
use serde::{Deserialize, Serialize};
use tauri::Url;
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, net::TcpListener};
use tracing::debug;

const OAUTH_CLIENT_ID: &str = "J2hzqzCxch8hfOPRFNINOZV5Ma4X4BFdZpMjAVEW";
const AUTH_URL: &str = "https://auth.liquidbounce.net/application/o/authorize/";
const TOKEN_URL: &str = "https://auth.liquidbounce.net/application/o/token/";

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientAccount {
    #[serde(rename = "accessToken")]
    access_token: AccessToken,
    #[serde(rename = "expiresAt")]
    expires_at: u64, // SystemTime
    #[serde(rename = "refreshToken")]
    refresh_token: RefreshToken
}

impl ClientAccount {

    pub fn is_expired(&self) -> bool {
        self.expires_at < SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn authenticate_request(&self, request: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder> {
        if self.is_expired() {
            bail!("Token expired");
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

    pub async fn renew(self) -> Result<ClientAccount> {
        let client_id = ClientId::new(OAUTH_CLIENT_ID.to_string());
        let auth_url = AuthUrl::new(AUTH_URL.to_string())
            .context("Invalid authorization endpoint URL")?;
        let token_url = TokenUrl::new(TOKEN_URL.to_string())
            .context("Invalid token endpoint URL")?;

        let client = BasicClient::new(client_id)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url);

        let token = client
            .exchange_refresh_token(&self.refresh_token)
            .request_async(&reqwest::Client::new())
            .await?;

        let expires_at = SystemTime::now() + token.expires_in().context("Missing expires_in")?;
        Ok(ClientAccount {
            access_token: token.access_token().clone(),
            expires_at: expires_at
                .duration_since(UNIX_EPOCH)
                .context("Time went backwards")?
                .as_secs(),
            refresh_token: token.refresh_token().context("Missing refresh token")?.clone()
        })
    }

}

pub struct AccountAuthenticator;

impl AccountAuthenticator {
    
    pub async fn start_auth<F>(on_url: F) -> Result<ClientAccount> where F: Fn(&String) {
        // Create an OAuth2 client by specifying the client ID, client secret, authorization URL and token URL.
        let client_id = ClientId::new(OAUTH_CLIENT_ID.to_string());
        let auth_url = AuthUrl::new(AUTH_URL.to_string())
            .context("Invalid authorization endpoint URL")?;
        let token_url = TokenUrl::new(TOKEN_URL.to_string())
            .context("Invalid token endpoint URL")?;

        // Start a local server to receive the authorization code from the provider
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("Failed to bind to a random port")?;
        let local_addr = listener.local_addr().context("Failed to get the local address")?;
        let redirect_uri = RedirectUrl::new(format!("http://{}:{}/", local_addr.ip(), local_addr.port()))
            .context("Invalid redirect URL")?;

        // Create a client to perform the OAuth2 flow
        let client = BasicClient::new(client_id)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_uri);
        
        // Create an HTTP client to perform the HTTP request
        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("Client should build")?;

        // Generate the authorization URL to which we'll redirect the user.
        let (authorize_url, csrf_state) = client
            .authorize_url(CsrfToken::new_random)
            .url();

        on_url(&authorize_url.to_string());

        let (code, state) = {
            // A very naive implementation of the redirect server.
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let (code, state) = Self::handle_http_request(&mut stream).await?;
                    
                    // The server will terminate itself after collecting the first code.
                    break (code, state);
                }
            }
        };

        debug!("OAuth returned the following code:\n{}\n", code.secret());
        debug!(
            "OAuth returned the following state:\n{} (expected `{}`)\n",
            state.secret(),
            csrf_state.secret()
        );

        // Exchange the code with a token.
        let token = client.exchange_code(code)
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
            refresh_token: token.refresh_token().context("Missing refresh token")?.clone()
        })
    } 

    async fn handle_http_request(
        stream: &mut tokio::net::TcpStream,
    ) -> Result<(AuthorizationCode, CsrfToken)> {
        let (reader, mut writer) = stream.split();
        let mut reader = BufReader::new(reader);

        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;

        let redirect_url = request_line.split_whitespace().nth(1).unwrap();
        let url = Url::parse(&("http://127.0.0.1".to_string() + redirect_url)).unwrap();

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

        let message = "You have successfully authenticated. You can close this tab now.";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
            message.len(),
            message
        );
        writer.write_all(response.as_bytes()).await?;
        writer.flush().await?;

        Ok((code, state))
    }

}