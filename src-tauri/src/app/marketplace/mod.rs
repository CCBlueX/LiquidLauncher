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

//! Marketplace items shared with LiquidBounce.
//!
//! Both programs read and write `gameDir/<branch>/LiquidBounce/marketplace.json` and the items
//! installed beside it. The client owns the format, so everything here matches what it produces.

pub mod api;
mod install;
mod revisions;
mod staging;
mod subscriptions;
pub mod view;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::client_api::Client;
pub use revisions::install_target;
pub use staging::stage_addons;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
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
    pub item_type: ItemType,
}

/// The launcher's data directory and the branch whose game directory holds the items.
pub struct GameDir {
    data: PathBuf,
    branch: String,
}

impl GameDir {
    pub fn new(data: impl Into<PathBuf>, branch: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            branch: branch.into(),
        }
    }

    fn data(&self) -> &Path {
        &self.data
    }

    fn mods(&self) -> PathBuf {
        self.data.join("gameDir").join(&self.branch).join("mods")
    }

    fn client(&self) -> PathBuf {
        self.data
            .join("gameDir")
            .join(&self.branch)
            .join("LiquidBounce")
    }

    fn subscriptions(&self) -> PathBuf {
        self.client().join("marketplace.json")
    }

    fn item(&self, item_id: u32) -> PathBuf {
        self.client()
            .join("marketplace")
            .join("items")
            .join(item_id.to_string())
    }
}

/// An edit made while the game runs. LiquidBounce writes its subscriptions back when it exits, so
/// these are applied only then.
#[derive(Clone)]
enum Edit {
    Add(SubscribedItem),
    Remove,
}

/// The edits waiting for the game to exit, by item.
#[derive(Default, Clone)]
struct Queued(BTreeMap<u32, Edit>);

impl Queued {
    /// Queues `edit`, or drops both when it undoes the one queued.
    fn push(&mut self, item_id: u32, edit: Edit) {
        match (self.0.remove(&item_id), &edit) {
            (Some(Edit::Add(_)), Edit::Remove) | (Some(Edit::Remove), Edit::Add(_)) => {}
            _ => {
                self.0.insert(item_id, edit);
            }
        }
    }

    fn adding(&self) -> impl Iterator<Item = &SubscribedItem> {
        self.0.values().filter_map(|edit| match edit {
            Edit::Add(item) => Some(item),
            Edit::Remove => None,
        })
    }

    fn removing(&self, item_id: u32) -> bool {
        matches!(self.0.get(&item_id), Some(Edit::Remove))
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Default)]
struct Session {
    /// From the start of a launch until the game exits.
    running: bool,
    queued: Queued,
}

static SESSION: LazyLock<Mutex<Session>> = LazyLock::new(Default::default);

fn session() -> MutexGuard<'static, Session> {
    SESSION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn queued() -> Queued {
    session().queued.clone()
}

pub fn game_started() {
    session().running = true;
}

pub fn game_running() -> bool {
    session().running
}

/// Applies what was edited while the game ran. Called once it exited or failed to start; calling
/// it again does nothing.
pub async fn game_exited(game: &GameDir) {
    let queued = {
        let mut session = session();
        session.running = false;
        std::mem::take(&mut session.queued)
    };

    for (item_id, edit) in queued.0 {
        let applied = match edit {
            Edit::Add(item) => subscriptions::add(game, &item).await,
            Edit::Remove => uninstall(game, item_id).await,
        };
        if let Err(error) = applied {
            warn!("Failed to apply a marketplace edit: {:?}", error);
        }
    }
}

/// Queues `edit` while the game runs, and tells whether it did.
fn queue_while_running(item_id: u32, edit: impl FnOnce() -> Edit) -> bool {
    let mut session = session();
    if session.running {
        session.queued.push(item_id, edit());
    }
    session.running
}

/// Subscribes to `item` and installs `revision`, or, while the game runs, once it exited. Undoes a
/// removal still waiting for that.
pub async fn add(
    client: &Client,
    game: &GameDir,
    item: SubscribedItem,
    revision: Option<u32>,
) -> Result<()> {
    if queue_while_running(item.id, || Edit::Add(item.clone())) {
        return Ok(());
    }

    if let Some(revision) = revision {
        install::install(
            game,
            item.id,
            revision,
            &api::download_url(client, item.id, revision),
        )
        .await?;
    }

    // A game started during the download has read the subscriptions already, and writes them
    // back when it exits.
    if queue_while_running(item.id, || Edit::Add(item.clone())) {
        return Ok(());
    }
    subscriptions::add(game, &item).await
}

/// Removes an item, or, while the game runs, once it exited. Cancels an addition still waiting.
pub async fn remove(game: &GameDir, item_id: u32) -> Result<()> {
    if queue_while_running(item_id, || Edit::Remove) {
        return Ok(());
    }
    uninstall(game, item_id).await
}

/// Removes a subscription and everything it installed.
async fn uninstall(game: &GameDir, item_id: u32) -> Result<()> {
    install::remove_files(game, item_id).await;
    subscriptions::remove(game, item_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edit_while_the_game_runs_undoes_the_opposite_one() {
        let item = |id| {
            Edit::Add(SubscribedItem {
                name: format!("Item {id}"),
                id,
                item_type: ItemType::Theme,
            })
        };
        let edits = |queued: &Queued| -> Vec<(u32, bool)> {
            queued
                .0
                .iter()
                .map(|(id, edit)| (*id, matches!(edit, Edit::Add(_))))
                .collect()
        };

        let mut queued = Queued::default();
        queued.push(1, item(1));
        queued.push(1, item(1));
        queued.push(2, Edit::Remove);
        assert_eq!(edits(&queued), [(1, true), (2, false)]);

        queued.push(1, Edit::Remove);
        queued.push(2, item(2));
        assert_eq!(edits(&queued), []);
    }
}
