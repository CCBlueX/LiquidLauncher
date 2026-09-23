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

//! Marketplace subscriptions shared with LiquidBounce.
//!
//! Both programs read and write `gameDir/<branch>/LiquidBounce/marketplace.json`. The client owns
//! the format, so everything here matches what its `ConfigSystem` produces, and unknown keys are
//! preserved rather than dropped. Neither side locks the file; the last writer wins.

use anyhow::{Context, Result};
use backon::{ConstantBuilder, Retryable};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tracing::{info, warn};

use crate::utils::{download_file, zip_extract};

/// Filename prefix for add-on jars staged into the mods directory.
///
/// Matches what LiquidBounce's own `AddonInstaller` writes, so a jar staged by either program is
/// recognised by the other.
const ADDON_PREFIX: &str = "liquidbounce-addon-";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketplaceItemType {
    Config,
    Theme,
    Addon,
    Script,
    #[serde(other)]
    Other,
}

/// A subscription as read for display and staging. Nothing writes this back: edits change the raw
/// entries, so types and fields this launcher does not know survive.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedItem {
    pub name: String,
    pub id: u32,
    #[serde(rename = "type")]
    pub item_type: MarketplaceItemType,
}

fn client_dir(data: &Path, branch: &str) -> PathBuf {
    data.join("gameDir").join(branch).join("LiquidBounce")
}

fn subscriptions_path(data: &Path, branch: &str) -> PathBuf {
    client_dir(data, branch).join("marketplace.json")
}

fn item_dir(data: &Path, branch: &str, item_id: u32) -> PathBuf {
    client_dir(data, branch)
        .join("marketplace")
        .join("items")
        .join(item_id.to_string())
}

fn mods_dir(data: &Path, branch: &str) -> PathBuf {
    data.join("gameDir").join(branch).join("mods")
}

/// Reads the subscription list, returning empty if the client has never written the file.
pub async fn read_subscriptions(data: &Path, branch: &str) -> Result<Vec<SubscribedItem>> {
    let Some(root) = read_root(&subscriptions_path(data, branch)).await? else {
        return Ok(vec![]);
    };

    let Some(entries) = subscribed_entries(&root) else {
        return Ok(vec![]);
    };

    // One unreadable entry must not hide the rest.
    Ok(entries
        .iter()
        .filter_map(
            |entry| match serde_json::from_value::<SubscribedItem>(entry.clone()) {
                Ok(item) => Some(item),
                Err(error) => {
                    warn!("Skipping unreadable marketplace entry: {}", error);
                    None
                }
            },
        )
        .collect())
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

fn subscribed_entries(root: &Value) -> Option<&Vec<Value>> {
    root.get("value")?
        .as_array()?
        .iter()
        .find(|entry| is_subscribed_list(entry))?
        .get("value")?
        .as_array()
}

fn subscribed_entries_mut(root: &mut Value) -> Option<&mut Vec<Value>> {
    let values = root.get_mut("value")?.as_array_mut()?;
    let index = match values.iter().position(is_subscribed_list) {
        Some(index) => index,
        None => {
            values.push(json!({ "name": "subscribed", "value": [] }));
            values.len() - 1
        }
    };
    values[index].get_mut("value")?.as_array_mut()
}

fn is_subscribed_list(entry: &Value) -> bool {
    entry.get("name").and_then(Value::as_str) == Some("subscribed")
}

fn entry_id(entry: &Value) -> Option<u64> {
    entry.get("id").and_then(Value::as_u64)
}

/// Applies `edit` to the raw subscription entries and writes the file back. Everything `edit` does
/// not touch stays as the client wrote it.
async fn edit_subscriptions(
    data: &Path,
    branch: &str,
    edit: impl FnOnce(&mut Vec<Value>),
) -> Result<()> {
    // Commands run concurrently, and each would write back a file without the other's edit.
    static EDITING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _editing = EDITING.lock().await;

    let path = subscriptions_path(data, branch);

    // Re-read immediately before writing, so a value the client added is carried over rather than
    // replaced by whatever the launcher last saw.
    let mut root = read_root(&path)
        .await?
        .unwrap_or_else(|| json!({ "name": "marketplace", "value": [] }));
    let entries = subscribed_entries_mut(&mut root)
        .with_context(|| format!("Unexpected layout of {}", path.display()))?;
    edit(entries);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Write beside the target and rename, so an interrupted write cannot truncate the file the
    // client reads at startup.
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(&root)?).await?;
    fs::rename(&temporary, &path).await?;
    Ok(())
}

/// Adds `item` to the subscriptions unless it is already there.
pub async fn subscribe(data: &Path, branch: &str, item: &SubscribedItem) -> Result<()> {
    edit_subscriptions(data, branch, |entries| {
        if !entries
            .iter()
            .any(|entry| entry_id(entry) == Some(item.id.into()))
        {
            entries.push(json!({ "name": item.name, "id": item.id, "type": item.item_type }));
        }
    })
    .await
}

/// Numeric subdirectories of an item, the ones the client counts as revisions.
async fn revision_dirs(item_dir: &Path) -> Result<Vec<(u32, PathBuf)>> {
    let mut revisions = vec![];
    let mut entries = match fs::read_dir(item_dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(revisions),
        Err(error) => return Err(error.into()),
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let revision = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok());
        if let Some(revision) = revision.filter(|_| path.is_dir()) {
            revisions.push((revision, path));
        }
    }
    Ok(revisions)
}

/// Mirrors the client's `SubscribedItem.installedRevisionId`: the highest numeric directory.
async fn installed_revision(item_dir: &Path) -> Option<u32> {
    let revisions = revision_dirs(item_dir).await.ok()?;
    revisions.into_iter().map(|(revision, _)| revision).max()
}

/// Downloads and extracts a revision, then leaves it as the only one installed.
pub async fn install(
    data: &Path,
    branch: &str,
    item_id: u32,
    revision_id: u32,
    download_url: &str,
) -> Result<()> {
    let item_dir = item_dir(data, branch, item_id);

    // Named like the client's, which counts only numeric directories as installed and cleans up
    // dotted ones.
    let partial = item_dir.join(format!(".{revision_id}.part"));
    if partial.exists() {
        fs::remove_dir_all(&partial).await?;
    }
    fs::create_dir_all(&partial).await?;

    let extracted = async {
        let archive = download_file(download_url, |_, _| {}).await?;
        zip_extract(Cursor::new(archive), &partial).await
    }
    .await;
    if let Err(error) = extracted {
        let _ = fs::remove_dir_all(&partial).await;
        return Err(error.context(format!(
            "Failed to install revision {revision_id} of item {item_id}"
        )));
    }

    commit(&item_dir, &partial, revision_id).await?;
    info!("Installed marketplace item {item_id} revision {revision_id}");
    Ok(())
}

/// Moves `partial` in as the only revision. Like the client, old revisions are renamed away before
/// they are deleted, since a half-deleted one still counts as installed, and they go back when the
/// new one cannot take their place.
async fn commit(item_dir: &Path, partial: &Path, revision_id: u32) -> Result<()> {
    let mut retired = vec![];
    let committed = async {
        for (revision, dir) in revision_dirs(item_dir).await? {
            let old = item_dir.join(format!(".{revision}.old"));
            if old.exists() {
                fs::remove_dir_all(&old).await?;
            }
            rename(&dir, &old).await?;
            retired.push((dir, old));
        }
        rename(partial, &item_dir.join(revision_id.to_string())).await?;
        anyhow::Ok(())
    }
    .await;

    if let Err(error) = committed {
        for (dir, old) in retired {
            let _ = rename(&old, &dir).await;
        }
        let _ = fs::remove_dir_all(partial).await;
        return Err(error.context(format!("Failed to move {}", partial.display())));
    }

    for (_, old) in retired {
        if let Err(error) = fs::remove_dir_all(&old).await {
            warn!("Failed to remove {}: {}", old.display(), error);
        }
    }
    Ok(())
}

// Windows refuses to rename a file or directory while a virus scanner still reads it.
async fn rename(from: &Path, to: &Path) -> std::io::Result<()> {
    (|| fs::rename(from, to))
        .retry(
            ConstantBuilder::default()
                .with_delay(Duration::from_millis(100))
                .with_max_times(20),
        )
        .await
}

/// Removes a subscription and everything it installed.
pub async fn uninstall(data: &Path, branch: &str, item_id: u32) -> Result<()> {
    let item_dir = item_dir(data, branch, item_id);
    if item_dir.exists() {
        fs::remove_dir_all(&item_dir).await.ok();
    }

    edit_subscriptions(data, branch, |entries| {
        entries.retain(|entry| entry_id(entry) != Some(item_id.into()))
    })
    .await
}

/// Copies subscribed add-on jars into the mods directory.
///
/// Called after `clear_mods`, which wipes that directory on every launch.
pub async fn stage_addons(data: &Path, branch: &str) -> Result<()> {
    let items = read_subscriptions(data, branch).await?;
    let mods = mods_dir(data, branch);
    fs::create_dir_all(&mods).await?;

    for item in items {
        if item.item_type != MarketplaceItemType::Addon {
            continue;
        }

        let Some(revision_id) = installed_revision(&item_dir(data, branch, item.id)).await else {
            continue;
        };

        let revision_dir = item_dir(data, branch, item.id).join(revision_id.to_string());
        match stage_one(&revision_dir, item.id, revision_id, &mods).await {
            Ok(name) => info!("Staged add-on '{}' as {}", item.name, name),
            Err(error) => warn!("Failed to stage add-on '{}': {:?}", item.name, error),
        }
    }

    Ok(())
}

async fn stage_one(
    revision_dir: &Path,
    item_id: u32,
    revision_id: u32,
    mods: &Path,
) -> Result<String> {
    let installation = installation_dir(revision_dir)
        .await
        .with_context(|| format!("No installed files for add-on {item_id}"))?;

    let mut jars = vec![];
    let mut entries = fs::read_dir(&installation).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "jar") {
            jars.push(path);
        }
    }

    // The archive is required to hold exactly one jar; the API server rejects anything else on
    // upload, so more than one here means a hand-placed file.
    let jar = match jars.len() {
        1 => jars.remove(0),
        count => anyhow::bail!(
            "Expected exactly one jar in {}, found {count}",
            installation.display()
        ),
    };

    let name = format!("{ADDON_PREFIX}{item_id}-{revision_id}.jar");
    fs::copy(&jar, mods.join(&name)).await?;
    Ok(name)
}

/// Mirrors the client's `SubscribedItem.getInstallationFolder`: the revision directory, or the one
/// subdirectory inside it that actually holds files.
async fn installation_dir(revision_dir: &Path) -> Option<PathBuf> {
    if !revision_dir.is_dir() {
        return None;
    }

    if contains_file(revision_dir).await {
        return Some(revision_dir.to_path_buf());
    }

    let mut entries = fs::read_dir(revision_dir).await.ok()?;
    while let Some(entry) = entries.next_entry().await.ok()? {
        let path = entry.path();
        if path.is_dir() && contains_file(&path).await {
            return Some(path);
        }
    }

    None
}

async fn contains_file(dir: &Path) -> bool {
    let Ok(mut entries) = fs::read_dir(dir).await else {
        return false;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.path().is_file() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liquidlauncher-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn concurrent_edits_all_land() {
        let data = scratch("concurrent");
        let path = subscriptions_path(&data, "nextgen");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let subscribed: Vec<_> = (1..=8)
            .map(|id| json!({ "name": format!("Theme {id}"), "id": id, "type": "Theme" }))
            .collect();
        let file = json!({
            "name": "marketplace",
            "value": [{ "name": "subscribed", "value": subscribed }],
        });
        std::fs::write(&path, file.to_string()).unwrap();

        futures::future::try_join_all((1..=8).map(|id| uninstall(&data, "nextgen", id)))
            .await
            .unwrap();
        let left = read_subscriptions(&data, "nextgen").await.unwrap();
        assert!(left.is_empty());

        std::fs::remove_dir_all(&data).unwrap();
    }

    #[tokio::test]
    async fn edits_keep_unknown_types_fields_and_entries() {
        let data = scratch("subscriptions");
        let path = subscriptions_path(&data, "nextgen");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        let history = json!({ "name": "history", "value": [1, 2] });
        let script = json!({
            "name": "Scripts",
            "id": 12,
            "type": "Script",
            "installedRevisionId": 99,
            "pinned": true,
        });
        let unknown = json!({ "name": "Shaders", "id": 13, "type": "Shader" });
        let theme = json!({ "name": "Beautify", "id": 14, "type": "Theme" });
        let addon = json!({ "name": "Extras", "id": 15, "type": "Addon" });
        let file = |subscribed: Value| {
            json!({
                "name": "marketplace",
                "version": 3,
                "value": [history, { "name": "subscribed", "value": subscribed }],
            })
        };
        std::fs::write(
            &path,
            file(json!([script, unknown, "broken", theme])).to_string(),
        )
        .unwrap();

        uninstall(&data, "nextgen", 14).await.unwrap();
        for (name, id, item_type) in [
            ("Extras", 15, MarketplaceItemType::Addon),
            ("Renamed", 12, MarketplaceItemType::Script),
        ] {
            let item = SubscribedItem {
                name: name.to_string(),
                id,
                item_type,
            };
            subscribe(&data, "nextgen", &item).await.unwrap();
        }

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(written, file(json!([script, unknown, "broken", addon])));

        let items: Vec<_> = read_subscriptions(&data, "nextgen")
            .await
            .unwrap()
            .into_iter()
            .map(|item| (item.id, item.item_type))
            .collect();
        assert_eq!(
            items,
            [
                (12, MarketplaceItemType::Script),
                (13, MarketplaceItemType::Other),
                (15, MarketplaceItemType::Addon),
            ]
        );

        std::fs::remove_dir_all(&data).unwrap();
    }

    #[tokio::test]
    async fn an_older_revision_replaces_the_installed_ones() {
        let item = scratch("commit");
        for name in ["12", "4251", ".4251.old"] {
            std::fs::create_dir_all(item.join(name)).unwrap();
        }
        let partial = item.join(".731.part");
        std::fs::create_dir_all(&partial).unwrap();
        std::fs::write(partial.join("extras.jar"), b"").unwrap();

        commit(&item, &partial, 731).await.unwrap();

        let left: Vec<_> = std::fs::read_dir(&item)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, ["731"]);
        assert!(item.join("731").join("extras.jar").is_file());

        std::fs::remove_dir_all(&item).unwrap();
    }

    #[tokio::test]
    async fn installed_revision_is_the_highest_numeric_directory() {
        let item = scratch("installed");
        assert_eq!(installed_revision(&item.join("missing")).await, None);
        assert_eq!(installed_revision(&item).await, None);

        for name in ["12", "4251", "4331.part", "latest"] {
            std::fs::create_dir_all(item.join(name)).unwrap();
        }
        std::fs::write(item.join("9999"), b"").unwrap();
        std::fs::write(item.join("731.zip"), b"").unwrap();
        assert_eq!(installed_revision(&item).await, Some(4251));

        std::fs::remove_dir_all(&item).unwrap();
    }
}
