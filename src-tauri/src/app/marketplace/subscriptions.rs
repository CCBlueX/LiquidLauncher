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

//! The subscription list in `marketplace.json`. Unknown keys are preserved rather than dropped.
//! Neither program locks the file; the last writer wins.

use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use tokio::fs;
use tracing::warn;

use super::{GameDir, SubscribedItem};

/// The subscriptions, empty if the client has never written the file.
pub async fn read(game: &GameDir) -> Result<Vec<SubscribedItem>> {
    let Some(root) = read_root(&game.subscriptions()).await? else {
        return Ok(vec![]);
    };
    let Some(entries) = entries(&root) else {
        return Ok(vec![]);
    };

    // One unreadable entry must not hide the rest.
    Ok(entries
        .iter()
        .filter_map(|entry| {
            serde_json::from_value(entry.clone())
                .inspect_err(|error| warn!("Skipping unreadable marketplace entry: {}", error))
                .ok()
        })
        .collect())
}

/// Adds `item` unless it is already there.
pub async fn add(game: &GameDir, item: &SubscribedItem) -> Result<()> {
    edit(game, |entries| {
        if !entries
            .iter()
            .any(|entry| id(entry) == Some(item.id.into()))
        {
            entries.push(json!({ "name": item.name, "id": item.id, "type": item.item_type }));
        }
    })
    .await
}

pub async fn remove(game: &GameDir, item_id: u32) -> Result<()> {
    edit(game, |entries| {
        entries.retain(|entry| id(entry) != Some(item_id.into()))
    })
    .await
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

fn entries(root: &Value) -> Option<&Vec<Value>> {
    root.get("value")?
        .as_array()?
        .iter()
        .find(|entry| is_subscribed_list(entry))?
        .get("value")?
        .as_array()
}

fn entries_mut(root: &mut Value) -> Option<&mut Vec<Value>> {
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

fn id(entry: &Value) -> Option<u64> {
    entry.get("id").and_then(Value::as_u64)
}

/// Applies `edit` to the raw entries and writes the file back. Everything `edit` does not touch
/// stays as the client wrote it.
async fn edit(game: &GameDir, edit: impl FnOnce(&mut Vec<Value>)) -> Result<()> {
    // Commands run concurrently, and each would write back a file without the other's edit.
    static EDITING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _editing = EDITING.lock().await;

    let path = game.subscriptions();

    // Re-read immediately before writing, so a value the client added is carried over rather than
    // replaced by whatever the launcher last saw.
    let mut root = read_root(&path)
        .await?
        .unwrap_or_else(|| json!({ "name": "marketplace", "value": [] }));
    let entries = entries_mut(&mut root)
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

#[cfg(test)]
mod tests {
    use super::super::ItemType;
    use super::*;
    use crate::app::marketplace::install::tests::scratch;

    fn write(game: &GameDir, subscribed: Value) {
        let path = game.subscriptions();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, subscribed.to_string()).unwrap();
    }

    #[tokio::test]
    async fn concurrent_edits_all_land() {
        let game = GameDir::new(scratch("concurrent"), "nextgen");
        let subscribed: Vec<_> = (1..=8)
            .map(|id| json!({ "name": format!("Theme {id}"), "id": id, "type": "Theme" }))
            .collect();
        write(
            &game,
            json!({
                "name": "marketplace",
                "value": [{ "name": "subscribed", "value": subscribed }],
            }),
        );

        futures::future::try_join_all((1..=8).map(|id| remove(&game, id)))
            .await
            .unwrap();
        assert!(read(&game).await.unwrap().is_empty());

        std::fs::remove_dir_all(game.data()).unwrap();
    }

    #[tokio::test]
    async fn edits_keep_unknown_types_fields_and_entries() {
        let game = GameDir::new(scratch("subscriptions"), "nextgen");

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
        write(&game, file(json!([script, unknown, "broken", theme])));

        remove(&game, 14).await.unwrap();
        for (name, id, item_type) in [
            ("Extras", 15, ItemType::Addon),
            ("Renamed", 12, ItemType::Script),
        ] {
            let item = SubscribedItem {
                name: name.to_string(),
                id,
                item_type,
            };
            add(&game, &item).await.unwrap();
        }

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(game.subscriptions()).unwrap()).unwrap();
        assert_eq!(written, file(json!([script, unknown, "broken", addon])));

        let items: Vec<_> = read(&game)
            .await
            .unwrap()
            .into_iter()
            .map(|item| (item.id, item.item_type))
            .collect();
        assert_eq!(
            items,
            [
                (12, ItemType::Script),
                (13, ItemType::Other),
                (15, ItemType::Addon),
            ]
        );

        std::fs::remove_dir_all(game.data()).unwrap();
    }
}
