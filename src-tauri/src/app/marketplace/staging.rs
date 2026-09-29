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

//! Add-ons are ordinary Fabric mods: each launch copies the subscribed ones into the mods
//! directory, in the revision that fits the launched build.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{info, warn};

use super::api::{self, Revision};
use super::install::{self, addon_jar, installed_revision, rename};
use super::revisions::{display_version, pick, revisions, supports_addons};
use super::{subscriptions, GameDir, ItemType, SubscribedItem};
use crate::app::client_api::{Build, Client};
use crate::minecraft::progress::{ProgressReceiver, ProgressUpdate};

/// Filename prefix for add-on jars staged into the mods directory.
///
/// Matches what LiquidBounce's own `AddonInstaller` writes, so a jar staged by either program is
/// recognised by the other.
const ADDON_PREFIX: &str = "liquidbounce-addon-";

/// Installs and copies into the mods directory, for the launched build, the newest revision of
/// every subscribed add-on that fits it.
///
/// Called once `clear_mods` emptied that directory and the build's and the custom mods are back.
pub async fn stage_addons(
    client: &Client,
    data: &Path,
    build: &Build,
    progress: &impl ProgressReceiver,
) -> Result<()> {
    let game = GameDir::new(data, &build.branch);
    let addons: Vec<_> = subscriptions::read(&game)
        .await?
        .into_iter()
        .filter(|item| item.item_type == ItemType::Addon)
        .collect();
    if addons.is_empty() {
        return Ok(());
    }

    if !supports_addons(build) {
        let names: Vec<_> = addons.iter().map(|item| item.name.as_str()).collect();
        progress.log(&format!(
            "This build predates add-ons, so {} will not load.",
            names.join(", ")
        ));
        return Ok(());
    }

    let mods = game.mods();
    fs::create_dir_all(&mods).await?;
    let mut remembered = read_staged(data).await;
    let unchanged = remembered.clone();

    for item in &addons {
        let Some((staged, jar)) =
            staged_jar(client, &game, build, item, &remembered, progress).await
        else {
            continue;
        };

        match stage_one(&jar, item.id, staged.revision, &mods).await {
            Ok(name) => {
                info!("Staged add-on '{}' as {}", item.name, name);
                remember(&mut remembered, staged);
            }
            Err(error) => {
                warn!("Failed to stage add-on '{}': {:?}", item.name, error);
                progress.log(&format!("Could not install {}.", item.name));
            }
        }
    }

    if remembered != unchanged {
        if let Err(error) = write_staged(data, &remembered).await {
            warn!("Failed to remember the staged add-ons: {:?}", error);
        }
    }
    Ok(())
}

/// An add-on revision staged for a build with these versions.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct Staged {
    item: u32,
    liquidbounce: String,
    minecraft: String,
    revision: u32,
    version: String,
}

impl Staged {
    fn for_build(build: &Build, item: u32, revision: &Revision) -> Self {
        Staged {
            item,
            liquidbounce: build.lb_version.clone(),
            minecraft: build.mc_version.clone(),
            revision: revision.id,
            version: revision.version.clone(),
        }
    }

    fn is_for(&self, item: u32, liquidbounce: &str, minecraft: &str) -> bool {
        self.item == item && self.liquidbounce == liquidbounce && self.minecraft == minecraft
    }
}

/// The launcher's own record, the client never reads it.
fn staged_path(data: &Path) -> PathBuf {
    data.join("staged_addons.json")
}

async fn read_staged(data: &Path) -> Vec<Staged> {
    let Ok(raw) = fs::read(staged_path(data)).await else {
        return vec![];
    };
    serde_json::from_slice(&raw)
        .inspect_err(|error| warn!("Failed to read the staged add-ons: {}", error))
        .unwrap_or_default()
}

async fn write_staged(data: &Path, staged: &[Staged]) -> Result<()> {
    fs::write(staged_path(data), serde_json::to_vec_pretty(staged)?).await?;
    Ok(())
}

fn last_staged<'a>(
    staged: &'a [Staged],
    item: u32,
    liquidbounce: &str,
    minecraft: &str,
) -> Option<&'a Staged> {
    staged
        .iter()
        .find(|staged| staged.is_for(item, liquidbounce, minecraft))
}

fn remember(remembered: &mut Vec<Staged>, staged: Staged) {
    remembered.retain(|other| !other.is_for(staged.item, &staged.liquidbounce, &staged.minecraft));
    remembered.push(staged);
}

/// Picks the revision of `item` to stage, or without the marketplace the one staged for the
/// build's versions last time. Makes it the installed one and finds its jar, or logs why there is
/// none. Other installed files stay, another build may take them.
async fn staged_jar(
    client: &Client,
    game: &GameDir,
    build: &Build,
    item: &SubscribedItem,
    remembered: &[Staged],
    progress: &impl ProgressReceiver,
) -> Option<(Staged, PathBuf)> {
    let item_dir = game.item(item.id);
    let installed = installed_revision(&item_dir).await;

    let (target, fallback) = match revisions(client, build, item.id).await {
        Ok((fitting, newest)) => {
            let (target, fallback) = pick(
                item,
                &build.lb_version,
                &fitting,
                newest.as_ref(),
                installed,
                progress,
            )?;
            let staged = |revision| Staged::for_build(build, item.id, revision);
            (staged(target), fallback.map(staged))
        }
        // The marketplace decides the fit by the build's LiquidBounce and Minecraft version alone.
        Err(error) => {
            warn!("Failed to look up add-on '{}': {:?}", item.name, error);
            let last = last_staged(remembered, item.id, &build.lb_version, &build.mc_version);
            let Some(last) = last else {
                progress.log(&format!("Could not look up {}.", item.name));
                return None;
            };
            progress.log(&format!(
                "Could not look up {}, staging {} as last time.",
                item.name,
                display_version(&last.version)
            ));
            (last.clone(), None)
        }
    };

    let revision_dir = |revision: u32| item_dir.join(revision.to_string());
    if installed == Some(target.revision) {
        match addon_jar(&revision_dir(target.revision)).await {
            Ok(jar) => return Some((target, jar)),
            Err(error) => warn!("Reinstalling add-on '{}': {:?}", item.name, error),
        }
    }

    progress.progress_update(ProgressUpdate::set_label(format!(
        "Installing add-on {}",
        item.name
    )));
    let url = api::download_url(client, item.id, target.revision);
    let chosen = match install::install(game, item.id, target.revision, &url).await {
        Ok(()) => Some(target),
        Err(error) => {
            warn!("Failed to install add-on '{}': {:?}", item.name, error);
            fallback
        }
    };

    let staged = match chosen {
        Some(chosen) => addon_jar(&revision_dir(chosen.revision))
            .await
            .inspect_err(|error| warn!("Failed to read add-on '{}': {:?}", item.name, error))
            .ok()
            .map(|jar| (chosen, jar)),
        None => None,
    };
    if staged.is_none() {
        progress.log(&format!("Could not install {}.", item.name));
    }
    staged
}

async fn stage_one(jar: &Path, item_id: u32, revision_id: u32, mods: &Path) -> Result<String> {
    let name = format!("{ADDON_PREFIX}{item_id}-{revision_id}.jar");

    // Fabric ignores non-jars, so a failed copy leaves no truncated jar behind.
    let part = mods.join(format!("{name}.part"));
    let copied = async {
        fs::copy(jar, &part).await?;
        rename(&part, &mods.join(&name)).await
    }
    .await;
    if let Err(error) = copied {
        let _ = fs::remove_file(&part).await;
        return Err(error.into());
    }

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_what_fit_the_same_build_last_time() {
        let staged = |item, minecraft: &str, revision| Staged {
            item,
            liquidbounce: "0.40.1".to_owned(),
            minecraft: minecraft.to_owned(),
            revision,
            version: format!("1.{revision}.0"),
        };
        let mut last = vec![];
        remember(&mut last, staged(1, "26.2", 1));
        remember(&mut last, staged(1, "26.3", 2));
        remember(&mut last, staged(3, "26.2", 19));
        remember(&mut last, staged(1, "26.2", 3));
        assert_eq!(last.len(), 3);

        let revision = |item, liquidbounce, minecraft| {
            last_staged(&last, item, liquidbounce, minecraft).map(|staged| staged.revision)
        };
        assert_eq!(revision(1, "0.40.1", "26.2"), Some(3));
        assert_eq!(revision(1, "0.40.1", "26.3"), Some(2));
        assert_eq!(revision(3, "0.40.1", "26.2"), Some(19));
        assert_eq!(revision(3, "0.40.1", "26.3"), None);
        assert_eq!(revision(1, "0.41.0", "26.3"), None);
    }
}
