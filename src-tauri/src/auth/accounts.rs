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

//! The account list is LiquidBounce's own `accounts.json`, so the launcher and
//! the in-game account manager show the same accounts.
//!
//! Entries and fields the launcher does not know (bans, TheAltening accounts)
//! stay as LiquidBounce wrote them.

use std::mem;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, to_value, Map, Value};
use uuid::Uuid;

use super::auth_manager;
use super::shared::{read_root, write_root};
use crate::minecraft::auth::{offline_uuid, MinecraftAccount};
use crate::minecraft::game_dir::GameDir;

const LIST_NAME: &str = "Accounts";

const NOT_IN_LIST: &str = "This account is no longer in the list.";

#[derive(Serialize)]
pub struct AccountEntry {
    /// `<type>:<uuid>`
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub name: String,
    pub uuid: Uuid,
    pub favorite: bool,
    /// The account the launcher is signed in with.
    pub active: bool,
}

/// An entry of the list. `Other` is one the launcher cannot use, like a TheAltening account.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Entry {
    Account(StoredAccount),
    Other(Value),
}

impl Entry {
    fn account(&self) -> Option<&StoredAccount> {
        match self {
            Self::Account(account) => Some(account),
            Self::Other(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum StoredAccount {
    #[serde(rename = "MicrosoftAccount", rename_all = "camelCase")]
    Microsoft {
        name: String,
        uuid: Uuid,
        auth_manager: Value,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename = "CrackedAccount")]
    Offline {
        name: String,
        /// LiquidBounce saves it after the first login.
        #[serde(skip_serializing_if = "Option::is_none")]
        uuid: Option<Uuid>,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename = "SessionAccount", rename_all = "camelCase")]
    Session {
        name: String,
        uuid: Uuid,
        access_token: String,
        #[serde(flatten)]
        extra: Extra,
    },
}

/// What LiquidBounce keeps next to every account, such as its bans.
#[derive(Default, Serialize, Deserialize)]
struct Extra {
    #[serde(default)]
    favorite: bool,
    #[serde(flatten)]
    unknown: Map<String, Value>,
}

impl StoredAccount {
    /// Type as the frontend names it, name and UUID.
    fn profile(&self) -> (&'static str, &str, Uuid) {
        match self {
            Self::Microsoft { name, uuid, .. } => ("Microsoft", name, *uuid),
            Self::Offline { name, uuid, .. } => {
                ("Offline", name, uuid.unwrap_or_else(|| offline_uuid(name)))
            }
            Self::Session { name, uuid, .. } => ("Session", name, *uuid),
        }
    }

    fn id(&self) -> String {
        let (kind, _, uuid) = self.profile();
        format!("{kind}:{uuid}")
    }

    fn extra(&self) -> &Extra {
        match self {
            Self::Microsoft { extra, .. }
            | Self::Offline { extra, .. }
            | Self::Session { extra, .. } => extra,
        }
    }

    fn extra_mut(&mut self) -> &mut Extra {
        match self {
            Self::Microsoft { extra, .. }
            | Self::Offline { extra, .. }
            | Self::Session { extra, .. } => extra,
        }
    }

    fn from_launcher(account: &MinecraftAccount) -> Result<Self> {
        Ok(match account {
            MinecraftAccount::MsaAccount { state, name, id } => Self::Microsoft {
                name: name.clone(),
                uuid: *id,
                auth_manager: auth_manager::to_java(state)?,
                extra: Extra::default(),
            },
            MinecraftAccount::OfflineAccount { name, id } => Self::Offline {
                name: name.clone(),
                uuid: Some(*id),
                extra: Extra::default(),
            },
            MinecraftAccount::SessionAccount { name, id, token } => Self::Session {
                name: name.clone(),
                uuid: *id,
                access_token: token.clone(),
                extra: Extra::default(),
            },
        })
    }

    fn into_launcher(self) -> Result<MinecraftAccount> {
        let (_, _, id) = self.profile();

        Ok(match self {
            Self::Microsoft {
                name, auth_manager, ..
            } => MinecraftAccount::MsaAccount {
                state: auth_manager::to_rust(&auth_manager)?,
                name,
                id,
            },
            Self::Offline { name, .. } => MinecraftAccount::OfflineAccount { name, id },
            Self::Session {
                name, access_token, ..
            } => MinecraftAccount::SessionAccount {
                name,
                id,
                token: access_token,
            },
        })
    }
}

/// `signed_in` joins the list when an older launcher signed it in without listing it.
pub async fn list(
    data: &Path,
    signed_in: Option<&MinecraftAccount>,
) -> Result<Vec<AccountEntry>> {
    let active = signed_in.map(launcher_id);
    let mut entries = read(data).await?;

    if let (Some(account), Some(id)) = (signed_in, &active) {
        if position(&entries, id).is_none() {
            add(data, account).await?;
            entries = read(data).await?;
        }
    }

    Ok(entries
        .iter()
        .filter_map(Entry::account)
        .map(|account| {
            let (kind, name, uuid) = account.profile();
            let id = account.id();

            AccountEntry {
                active: active.as_deref() == Some(id.as_str()),
                id,
                kind,
                name: name.to_string(),
                uuid,
                favorite: account.extra().favorite,
            }
        })
        .collect())
}

pub async fn has_microsoft(data: &Path) -> Result<bool> {
    let entries = read(data).await?;
    Ok(entries
        .iter()
        .any(|entry| matches!(entry.account(), Some(StoredAccount::Microsoft { .. }))))
}

pub async fn select(data: &Path, id: &str) -> Result<MinecraftAccount> {
    let entries = read(data).await?;
    entries
        .into_iter()
        .find_map(|entry| match entry {
            Entry::Account(account) if account.id() == id => Some(account),
            _ => None,
        })
        .context(NOT_IN_LIST)?
        .into_launcher()
}

/// Adds the account, or updates its entry when it is in the list already.
pub async fn add(data: &Path, account: &MinecraftAccount) -> Result<()> {
    let mut account = StoredAccount::from_launcher(account)?;

    edit(data, |entries| {
        match account_mut(entries, &account.id()) {
            Some(listed) => {
                *account.extra_mut() = mem::take(listed.extra_mut());
                *listed = account;
            }
            None => {
                let bans = ("bans".to_string(), json!({}));
                account.extra_mut().unknown.extend([bans]);
                entries.push(Entry::Account(account));
            }
        }
        Ok(())
    })
    .await
}

pub async fn remove(data: &Path, id: &str) -> Result<()> {
    edit(data, |entries| {
        let index = position(entries, id).context(NOT_IN_LIST)?;
        entries.remove(index);
        Ok(())
    })
    .await
}

pub async fn set_favorite(data: &Path, id: &str, favorite: bool) -> Result<()> {
    edit(data, |entries| {
        let account = account_mut(entries, id).context(NOT_IN_LIST)?;
        account.extra_mut().favorite = favorite;
        Ok(())
    })
    .await
}

fn path(data: &Path) -> PathBuf {
    GameDir::new(data).client().join("accounts.json")
}

async fn read(data: &Path) -> Result<Vec<Entry>> {
    let Some(mut root) = read_root(&path(data)).await? else {
        return Ok(Vec::new());
    };
    match list_in(&mut root) {
        Some(list) => Ok(from_value(list.take())?),
        None => Ok(Vec::new()),
    }
}

async fn edit(data: &Path, change: impl FnOnce(&mut Vec<Entry>) -> Result<()>) -> Result<()> {
    let path = path(data);
    let mut root = match read_root(&path).await? {
        Some(root) => root,
        None => json!({ "name": LIST_NAME, "value": [{ "name": LIST_NAME, "value": [] }] }),
    };
    let list =
        list_in(&mut root).with_context(|| format!("Unexpected layout of {}", path.display()))?;

    let mut entries = from_value(list.take())?;
    change(&mut entries)?;
    *list = to_value(entries)?;
    write_root(&path, &root).await
}

/// `{"name": "Accounts", "value": [{"name": "Accounts", "value": [<entry>, ...]}]}`
fn list_in(root: &mut Value) -> Option<&mut Value> {
    root.get_mut("value")?
        .as_array_mut()?
        .iter_mut()
        .find(|value| value["name"] == LIST_NAME)?
        .get_mut("value")
}

/// What [`StoredAccount::id`] is once the account is in the list.
fn launcher_id(account: &MinecraftAccount) -> String {
    let (kind, id) = match account {
        MinecraftAccount::MsaAccount { id, .. } => ("Microsoft", id),
        MinecraftAccount::OfflineAccount { id, .. } => ("Offline", id),
        MinecraftAccount::SessionAccount { id, .. } => ("Session", id),
    };
    format!("{kind}:{id}")
}

fn position(entries: &[Entry], id: &str) -> Option<usize> {
    entries
        .iter()
        .position(|entry| entry.account().is_some_and(|account| account.id() == id))
}

fn account_mut<'a>(entries: &'a mut [Entry], id: &str) -> Option<&'a mut StoredAccount> {
    entries.iter_mut().find_map(|entry| match entry {
        Entry::Account(account) if account.id() == id => Some(account),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use minecraft_auth::java::JavaAuthManager;
    use serde_json::from_str;
    use tempfile::{tempdir, TempDir};

    use super::*;
    use crate::auth::auth_manager::tests::{client, java_sample};

    const MICROSOFT_UUID: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
    const STEVE_UUID: &str = "5627dd98-e6be-3c21-b8a8-e92344183641";
    const SESSION_UUID: &str = "853c80ef-3c37-49fd-aa49-938b674adae6";

    fn altening() -> Value {
        json!({
            "type": "AlteningAccount",
            "name": "alt@alt.com",
            "uuid": "11111111-2222-3333-4444-555555555555",
            "accessToken": "altening-token",
            "favorite": false,
            "bans": {}
        })
    }

    /// As LiquidBounce writes it. Alex has no UUID yet and jeb_ an undashed one.
    fn liquidbounce_file() -> Value {
        file_with(json!([
            {
                "name": "Notch",
                "uuid": MICROSOFT_UUID,
                "authManager": java_sample(),
                "type": "MicrosoftAccount",
                "favorite": true,
                "bans": { "play.example.net": { "permanent": true } }
            },
            altening(),
            {
                "name": "Steve",
                "uuid": STEVE_UUID,
                "online": false,
                "type": "CrackedAccount",
                "favorite": false,
                "bans": {}
            },
            {
                "name": "Alex",
                "online": false,
                "type": "CrackedAccount",
                "favorite": false,
                "bans": {}
            },
            {
                "name": "jeb_",
                "uuid": SESSION_UUID.replace('-', ""),
                "accessToken": "session-token",
                "type": "SessionAccount",
                "favorite": false,
                "bans": {}
            }
        ]))
    }

    fn file_with(accounts: Value) -> Value {
        json!({ "name": "Accounts", "value": [{ "name": "Accounts", "value": accounts }] })
    }

    fn data_with(content: &str) -> TempDir {
        let data = tempdir().unwrap();
        fs::create_dir_all(path(data.path()).parent().unwrap()).unwrap();
        fs::write(path(data.path()), content).unwrap();
        data
    }

    fn file(data: &Path) -> Value {
        from_str(&fs::read_to_string(path(data)).unwrap()).unwrap()
    }

    fn file_accounts(data: &Path) -> Vec<Value> {
        file(data)["value"][0]["value"].as_array().unwrap().clone()
    }

    fn offline(name: &str) -> MinecraftAccount {
        MinecraftAccount::auth_offline(name.to_string())
    }

    fn microsoft() -> MinecraftAccount {
        MinecraftAccount::MsaAccount {
            state: auth_manager::to_rust(&java_sample()).unwrap(),
            name: "Notch".to_string(),
            id: Uuid::parse_str(MICROSOFT_UUID).unwrap(),
        }
    }

    #[tokio::test]
    async fn lists_what_liquidbounce_writes() {
        let data = data_with(&liquidbounce_file().to_string());

        let entries = list(data.path(), Some(&microsoft())).await.unwrap();
        let ids: Vec<_> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                format!("Microsoft:{MICROSOFT_UUID}"),
                format!("Offline:{STEVE_UUID}"),
                format!("Offline:{}", offline_uuid("Alex")),
                format!("Session:{SESSION_UUID}"),
            ]
        );
        assert_eq!(
            to_value(&entries[0]).unwrap(),
            json!({
                "id": format!("Microsoft:{MICROSOFT_UUID}"),
                "type": "Microsoft",
                "name": "Notch",
                "uuid": MICROSOFT_UUID,
                "favorite": true,
                "active": true
            })
        );
        assert!(entries[1..].iter().all(|entry| !entry.active));
        assert_eq!(file(data.path()), liquidbounce_file());
    }

    #[tokio::test]
    async fn the_signed_in_account_joins_the_list() {
        let data = tempdir().unwrap();

        let entries = list(data.path(), Some(&offline("Steve"))).await.unwrap();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].active);
        assert_eq!(file_accounts(data.path())[0]["uuid"], STEVE_UUID);
    }

    #[tokio::test]
    async fn hides_entries_it_cannot_use() {
        let data = data_with(
            &file_with(json!([
                { "type": "MicrosoftAccount", "name": "Notch", "uuid": MICROSOFT_UUID },
                { "type": "SessionAccount", "name": "jeb_", "accessToken": "token" },
                { "type": "SessionAccount", "uuid": SESSION_UUID, "accessToken": "token" },
                altening()
            ]))
            .to_string(),
        );

        assert!(list(data.path(), None).await.unwrap().is_empty());
        assert!(!has_microsoft(data.path()).await.unwrap());
    }

    #[tokio::test]
    async fn empty_without_a_file() {
        let data = tempdir().unwrap();

        assert!(list(data.path(), None).await.unwrap().is_empty());
        assert!(!has_microsoft(data.path()).await.unwrap());
    }

    #[tokio::test]
    async fn empty_file_is_a_new_list_but_a_broken_one_is_left_alone() {
        let data = data_with(" \n");
        let data = data.path();

        assert!(list(data, None).await.unwrap().is_empty());
        add(data, &offline("Steve")).await.unwrap();
        assert_eq!(file_accounts(data).len(), 1);

        fs::write(path(data), "{ broken").unwrap();
        assert!(list(data, None).await.is_err());
        assert!(add(data, &offline("Alex")).await.is_err());
        assert_eq!(fs::read_to_string(path(data)).unwrap(), "{ broken");
    }

    #[tokio::test]
    async fn selects_accounts_the_launcher_can_start() {
        let data = data_with(&liquidbounce_file().to_string());
        let data = data.path();

        let microsoft = select(data, &format!("Microsoft:{MICROSOFT_UUID}")).await;
        let MinecraftAccount::MsaAccount { state, name, id } = microsoft.unwrap() else {
            panic!("expected a Microsoft account");
        };
        assert_eq!(
            (name.as_str(), id.to_string()),
            ("Notch", MICROSOFT_UUID.to_string())
        );
        JavaAuthManager::from_json(client(), &state).unwrap();

        let session = select(data, &format!("Session:{SESSION_UUID}")).await;
        let MinecraftAccount::SessionAccount { token, .. } = session.unwrap() else {
            panic!("expected a session account");
        };
        assert_eq!(token, "session-token");

        let unknown = select(data, "Offline:00000000-0000-0000-0000-000000000000").await;
        assert!(unknown.is_err());
    }

    #[tokio::test]
    async fn creates_the_file_liquidbounce_expects() {
        let data = tempdir().unwrap();

        add(data.path(), &offline("Steve")).await.unwrap();

        assert_eq!(
            file(data.path()),
            file_with(json!([{
                "type": "CrackedAccount",
                "name": "Steve",
                "uuid": STEVE_UUID,
                "favorite": false,
                "bans": {}
            }]))
        );
    }

    #[tokio::test]
    async fn edits_keep_unknown_fields_and_entries() {
        let mut original = liquidbounce_file();
        original["extra"] = json!("kept");
        let data = data_with(&original.to_string());
        let data = data.path();

        let herobrine = format!("Offline:{}", offline_uuid("Herobrine"));
        add(data, &offline("Herobrine")).await.unwrap();
        set_favorite(data, &herobrine, true).await.unwrap();
        remove(data, &format!("Offline:{STEVE_UUID}")).await.unwrap();

        assert_eq!(file(data)["extra"], "kept");
        let accounts = file_accounts(data);
        assert_eq!(accounts.len(), 5);
        assert_eq!(accounts[0], original["value"][0]["value"][0]);
        assert_eq!(accounts[1], altening());
        assert_eq!(
            accounts[4],
            json!({
                "type": "CrackedAccount",
                "name": "Herobrine",
                "uuid": offline_uuid("Herobrine"),
                "favorite": true,
                "bans": {}
            })
        );
    }

    #[tokio::test]
    async fn adding_an_account_again_updates_its_entry() {
        let data = data_with(&liquidbounce_file().to_string());

        add(data.path(), &microsoft()).await.unwrap();

        let accounts = file_accounts(data.path());
        assert_eq!(accounts.len(), 5);
        let entry = &accounts[0];
        assert_eq!(entry["favorite"], true);
        assert_eq!(entry["bans"]["play.example.net"]["permanent"], true);
        assert_eq!(entry["authManager"]["deviceKeyPair"]["algorithm"], "EC");
        assert!(entry["authManager"].get("minecraftToken").is_none());
    }

    #[tokio::test]
    async fn only_a_microsoft_account_counts_as_microsoft() {
        let data = tempdir().unwrap();

        add(data.path(), &offline("Steve")).await.unwrap();
        assert!(!has_microsoft(data.path()).await.unwrap());

        add(data.path(), &microsoft()).await.unwrap();
        assert!(has_microsoft(data.path()).await.unwrap());
    }
}
