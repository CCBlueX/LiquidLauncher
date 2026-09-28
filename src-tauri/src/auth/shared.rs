/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2026 CCBlueX
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

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oauth2::{AccessToken, RefreshToken};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::fs;

use super::ClientAccount;
use crate::app::client_api_target::CLIENT_BRANCH;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    access_token: Expiring,
    refresh_token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expiring {
    value: String,
    expires_at: u64,
}

pub async fn read(data: &Path) -> Result<Option<ClientAccount>> {
    let path = path(data);
    let Some(root) = read_root(&path).await? else {
        return Ok(None);
    };
    let Some(session) = account(&root).and_then(|account| account.get("session")) else {
        return Ok(None);
    };
    let session = Session::deserialize(session)
        .with_context(|| format!("Unexpected session in {}", path.display()))?;

    Ok(Some(ClientAccount {
        access_token: AccessToken::new(session.access_token.value),
        expires_at: session.access_token.expires_at / 1000,
        refresh_token: RefreshToken::new(session.refresh_token),
    }))
}

pub async fn write(data: &Path, account: Option<&ClientAccount>) -> Result<()> {
    let path = path(data);

    let mut root = read_root(&path)
        .await?
        .unwrap_or_else(|| json!({ "name": "account", "value": [] }));
    let entry = account_mut(&mut root)
        .with_context(|| format!("Unexpected layout of {}", path.display()))?;

    match account {
        Some(account) => {
            let session = Session {
                access_token: Expiring {
                    value: account.access_token.secret().clone(),
                    expires_at: account.expires_at * 1000,
                },
                refresh_token: account.refresh_token.secret().clone(),
            };
            entry.insert("session".to_string(), serde_json::to_value(session)?);
        }
        None => {
            entry.remove("session");
        }
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec(&root)?).await?;
    fs::rename(&temporary, &path).await?;
    Ok(())
}

pub async fn renew(data: &Path, account: ClientAccount) -> Result<ClientAccount> {
    let account = account.renew().await?;
    write(data, Some(&account)).await?;
    Ok(account)
}

fn path(data: &Path) -> PathBuf {
    data.join("gameDir")
        .join(CLIENT_BRANCH)
        .join("LiquidBounce")
        .join("account.json")
}

async fn read_root(path: &Path) -> Result<Option<Value>> {
    let raw = match fs::read_to_string(path).await {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("Failed to read {}", path.display()))
        }
    };

    serde_json::from_str(&raw)
        .map(Some)
        .with_context(|| format!("Failed to parse {}", path.display()))
}

fn account(root: &Value) -> Option<&Value> {
    root.get("value")?
        .as_array()?
        .iter()
        .find(|entry| is_account(entry))?
        .get("value")
}

fn account_mut(root: &mut Value) -> Option<&mut Map<String, Value>> {
    let values = root.get_mut("value")?.as_array_mut()?;
    let index = match values.iter().position(is_account) {
        Some(index) => index,
        None => {
            values.push(json!({ "name": "account", "value": {} }));
            values.len() - 1
        }
    };
    values[index].get_mut("value")?.as_object_mut()
}

fn is_account(entry: &Value) -> bool {
    entry.get("name").and_then(Value::as_str) == Some("account")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "liquidlauncher-account-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn account(access: &str, refresh: &str, expires_at: u64) -> ClientAccount {
        ClientAccount {
            access_token: AccessToken::new(access.to_string()),
            expires_at,
            refresh_token: RefreshToken::new(refresh.to_string()),
        }
    }

    fn file(data: &Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path(data)).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn reads_what_the_client_writes() {
        let data = scratch("client");
        std::fs::create_dir_all(path(&data).parent().unwrap()).unwrap();
        std::fs::write(
            path(&data),
            r#"{"name":"account","value":[{"name":"account","value":{"session":{"accessToken":{"value":"access","expiresAt":1790000000123},"refreshToken":"refresh"}}}]}"#,
        )
        .unwrap();

        let session = read(&data).await.unwrap().unwrap();
        assert_eq!(session.access_token.secret(), "access");
        assert_eq!(session.expires_at, 1790000000);
        assert_eq!(session.refresh_token.secret(), "refresh");

        std::fs::remove_dir_all(&data).unwrap();
    }

    #[tokio::test]
    async fn signed_out_without_a_file_or_session() {
        let data = scratch("signed-out");
        assert!(read(&data).await.unwrap().is_none());

        std::fs::create_dir_all(path(&data).parent().unwrap()).unwrap();
        std::fs::write(
            path(&data),
            r#"{"name":"account","value":[{"name":"account","value":{}}]}"#,
        )
        .unwrap();
        assert!(read(&data).await.unwrap().is_none());

        std::fs::remove_dir_all(&data).unwrap();
    }

    #[tokio::test]
    async fn writes_what_the_client_reads() {
        let data = scratch("write");

        write(&data, Some(&account("access", "refresh", 1790000000)))
            .await
            .unwrap();
        assert_eq!(
            file(&data),
            json!({
                "name": "account",
                "value": [{
                    "name": "account",
                    "value": {
                        "session": {
                            "accessToken": { "value": "access", "expiresAt": 1790000000000u64 },
                            "refreshToken": "refresh",
                        },
                    },
                }],
            })
        );

        write(&data, None).await.unwrap();
        assert_eq!(
            file(&data),
            json!({ "name": "account", "value": [{ "name": "account", "value": {} }] })
        );

        std::fs::remove_dir_all(&data).unwrap();
    }

    #[tokio::test]
    async fn writes_keep_unknown_fields_and_entries() {
        let data = scratch("unknown");
        std::fs::create_dir_all(path(&data).parent().unwrap()).unwrap();
        let other = json!({ "name": "other", "value": 1 });
        std::fs::write(
            path(&data),
            json!({
                "name": "account",
                "value": [other, { "name": "account", "value": { "extra": true } }],
            })
            .to_string(),
        )
        .unwrap();

        write(&data, Some(&account("access", "refresh", 1)))
            .await
            .unwrap();
        write(&data, None).await.unwrap();

        assert_eq!(
            file(&data),
            json!({
                "name": "account",
                "value": [other, { "name": "account", "value": { "extra": true } }],
            })
        );

        std::fs::remove_dir_all(&data).unwrap();
    }
}
