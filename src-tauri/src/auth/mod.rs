use anyhow::Result;
use oauth2::{basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};
use tauri::Url;
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, net::TcpListener};
use tracing::info;

pub async fn auth_with_liquidbounce() -> Result<()> {
    let client_id = ClientId::new(env!("OAUTH_CLIENT_ID").to_string());
    let auth_url = AuthUrl::new("https://auth.liquidbounce.net/application/o/authorize/".to_string())
        .expect("Invalid authorization endpoint URL");

    let token_url = TokenUrl::new("https://auth.liquidbounce.net/application/o/token/".to_string())
        .expect("Invalid token endpoint URL");

    let client_secret = ClientSecret::new(env!("OAUTH_CLIENT_SECRET").to_string());
    
    let client = BasicClient::new(client_id)
        .set_client_secret(client_secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(
            RedirectUrl::new("http://localhost:49444".to_string()).expect("Invalid redirect URL"),
        );
    
    let http_client = reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build");

    // Generate the authorization URL to which we'll redirect the user.
    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("id".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    info!("Open this URL in your browser:\n{authorize_url}\n");

    let (code, state) = {
        // A very naive implementation of the redirect server.
        let listener = TcpListener::bind("127.0.0.1:49444").await.unwrap();

        loop {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut reader = BufReader::new(&mut stream);

                let mut request_line = String::new();
                reader.read_line(&mut request_line).await.unwrap();

                let redirect_url = request_line.split_whitespace().nth(1).unwrap();
                let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code = url
                    .query_pairs()
                    .find(|(key, _)| key == "code")
                    .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
                    .unwrap();

                let state = url
                    .query_pairs()
                    .find(|(key, _)| key == "state")
                    .map(|(_, state)| CsrfToken::new(state.into_owned()))
                    .unwrap();

                let message = "Go back to your terminal :)";
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                    message.len(),
                    message
                );
                stream.write_all(response.as_bytes()).await.unwrap();

                // The server will terminate itself after collecting the first code.
                break (code, state);
            }
        }
    };

    info!("OAuth returned the following code:\n{}\n", code.secret());
    info!(
        "OAuth returned the following state:\n{} (expected `{}`)\n",
        state.secret(),
        csrf_state.secret()
    );

    // Exchange the code with a token.
    let token_res = client.exchange_code(code)
        .request_async(&http_client).await;

    info!("OAuth returned the following token:\n{token_res:?}\n");

    if let Ok(token) = token_res {
        info!("OAuth token received: {:?}", token);
    }

    Ok(())
}