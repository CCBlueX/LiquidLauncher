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
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

use crate::utils::{download_file, zip_extract};
use crate::LAUNCHER_DIRECTORY;

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
    /// Also what the retired `Script` type deserializes to, matching the client.
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedItem {
    pub name: String,
    pub id: u32,
    #[serde(rename = "type")]
    pub item_type: MarketplaceItemType,
    #[serde(rename = "installedRevisionId")]
    pub installed_revision_id: Option<u32>,
}

fn client_dir(branch: &str) -> PathBuf {
    LAUNCHER_DIRECTORY
        .data_dir()
        .join("gameDir")
        .join(branch)
        .join("LiquidBounce")
}

fn subscriptions_path(branch: &str) -> PathBuf {
    client_dir(branch).join("marketplace.json")
}

pub fn marketplace_root(branch: &str) -> PathBuf {
    client_dir(branch).join("marketplace")
}

fn mods_dir(branch: &str) -> PathBuf {
    LAUNCHER_DIRECTORY
        .data_dir()
        .join("gameDir")
        .join(branch)
        .join("mods")
}

/// Reads the subscription list, returning empty if the client has never written the file.
pub async fn read_subscriptions(branch: &str) -> Result<Vec<SubscribedItem>> {
    let path = subscriptions_path(branch);
    if !path.exists() {
        return Ok(vec![]);
    }

    let raw = fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let root: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let Some(array) = subscribed_array(&root) else {
        return Ok(vec![]);
    };

    // One unreadable entry must not hide the rest.
    Ok(array
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

fn subscribed_array(root: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
    root.get("value")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("name").and_then(|n| n.as_str()) == Some("subscribed"))?
        .get("value")?
        .as_array()
}

/// Writes the subscription list back, leaving every other key in the file untouched.
pub async fn write_subscriptions(branch: &str, items: &[SubscribedItem]) -> Result<()> {
    let path = subscriptions_path(branch);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Re-read immediately before writing, so a value the client added is carried over rather than
    // replaced by whatever the launcher last saw.
    let mut root: serde_json::Value = match fs::read_to_string(&path).await {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| default_root()),
        Err(_) => default_root(),
    };

    let encoded = serde_json::to_value(items)?;
    let mut replaced = false;

    if let Some(values) = root.get_mut("value").and_then(|v| v.as_array_mut()) {
        for entry in values.iter_mut() {
            if entry.get("name").and_then(|n| n.as_str()) == Some("subscribed") {
                entry["value"] = encoded.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            values.push(serde_json::json!({ "name": "subscribed", "value": encoded }));
        }
    } else {
        root = default_root();
        root["value"] = serde_json::json!([{ "name": "subscribed", "value": encoded }]);
    }

    // Write beside the target and rename, so an interrupted write cannot truncate the file the
    // client reads at startup.
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(&root)?).await?;
    fs::rename(&temporary, &path).await?;
    Ok(())
}

fn default_root() -> serde_json::Value {
    serde_json::json!({ "name": "marketplace", "value": [] })
}

/// Downloads and extracts a revision into the same layout the client uses, then records it.
pub async fn install(
    branch: &str,
    item: &SubscribedItem,
    revision_id: u32,
    download_url: &str,
) -> Result<()> {
    let revision_dir = marketplace_root(branch)
        .join("items")
        .join(item.id.to_string())
        .join(revision_id.to_string());

    if revision_dir.exists() {
        fs::remove_dir_all(&revision_dir).await.ok();
    }
    fs::create_dir_all(&revision_dir).await?;

    let archive = download_file(download_url, |_, _| {}).await?;
    zip_extract(Cursor::new(archive), &revision_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to extract revision {revision_id} of item {}",
                item.id
            )
        })?;

    let mut items = read_subscriptions(branch).await?;
    match items.iter_mut().find(|existing| existing.id == item.id) {
        Some(existing) => existing.installed_revision_id = Some(revision_id),
        None => {
            let mut stored = item.clone();
            stored.installed_revision_id = Some(revision_id);
            items.push(stored);
        }
    }
    write_subscriptions(branch, &items).await?;

    info!(
        "Installed marketplace item {} revision {}",
        item.id, revision_id
    );
    Ok(())
}

/// Removes a subscription and everything it installed.
pub async fn uninstall(branch: &str, item_id: u32) -> Result<()> {
    let item_dir = marketplace_root(branch)
        .join("items")
        .join(item_id.to_string());
    if item_dir.exists() {
        fs::remove_dir_all(&item_dir).await.ok();
    }

    let items: Vec<SubscribedItem> = read_subscriptions(branch)
        .await?
        .into_iter()
        .filter(|item| item.id != item_id)
        .collect();
    write_subscriptions(branch, &items).await
}

/// Copies subscribed add-on jars into the mods directory.
///
/// Called after `clear_mods`, which wipes that directory on every launch.
pub async fn stage_addons(branch: &str) -> Result<()> {
    let items = read_subscriptions(branch).await?;
    let mods = mods_dir(branch);
    fs::create_dir_all(&mods).await?;

    for item in items {
        if item.item_type != MarketplaceItemType::Addon {
            continue;
        }

        let Some(revision_id) = item.installed_revision_id else {
            continue;
        };

        match stage_one(branch, item.id, revision_id, &mods).await {
            Ok(name) => info!("Staged add-on '{}' as {}", item.name, name),
            Err(error) => warn!("Failed to stage add-on '{}': {:?}", item.name, error),
        }
    }

    Ok(())
}

async fn stage_one(branch: &str, item_id: u32, revision_id: u32, mods: &Path) -> Result<String> {
    let installation = installation_dir(branch, item_id, revision_id)
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
async fn installation_dir(branch: &str, item_id: u32, revision_id: u32) -> Option<PathBuf> {
    let revision_dir = marketplace_root(branch)
        .join("items")
        .join(item_id.to_string())
        .join(revision_id.to_string());

    if !revision_dir.is_dir() {
        return None;
    }

    if contains_file(&revision_dir).await {
        return Some(revision_dir);
    }

    let mut entries = fs::read_dir(&revision_dir).await.ok()?;
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
