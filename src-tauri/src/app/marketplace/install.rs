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

//! Installed revisions on disk, laid out like the client's: `items/<id>/<revision>`, where only
//! numeric directories count and the highest is the installed one.

use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use backon::{ConstantBuilder, Retryable};
use serde_json::Value;
use tokio::fs;
use tracing::{info, warn};

use super::{GameDir, ItemType, SubscribedItem};
use crate::utils::{download_file, zip_extract};

/// The installed revision of an item and the version its files carry, if they name it.
pub struct Installed {
    pub revision: u32,
    pub version: Option<String>,
}

/// Reads what is installed of `item` from disk alone, and a theme's version from its
/// `metadata.json`.
pub async fn installed(game: &GameDir, item: &SubscribedItem) -> Option<Installed> {
    let item_dir = game.item(item.id);
    let revision = installed_revision(&item_dir).await?;

    let version = match item.item_type {
        ItemType::Theme => theme_version(&item_dir.join(revision.to_string())).await,
        _ => None,
    };
    Some(Installed { revision, version })
}

/// Mirrors the client's `SubscribedItem.installedRevisionId`: the highest numeric directory.
pub async fn installed_revision(item_dir: &Path) -> Option<u32> {
    let revisions = revision_dirs(item_dir).await.ok()?;
    revisions.into_iter().map(|(revision, _)| revision).max()
}

async fn theme_version(revision_dir: &Path) -> Option<String> {
    let metadata = installation_dir(revision_dir).await?.join("metadata.json");
    let metadata: Value = serde_json::from_slice(&fs::read(metadata).await.ok()?).ok()?;
    Some(metadata.get("version")?.as_str()?.to_owned())
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

/// Downloads and extracts a revision, then leaves it as the only one installed.
pub async fn install(
    game: &GameDir,
    item_id: u32,
    revision_id: u32,
    download_url: &str,
) -> Result<()> {
    let item_dir = game.item(item_id);

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

/// Windows refuses to rename a file or directory while a virus scanner still reads it.
pub async fn rename(from: &Path, to: &Path) -> std::io::Result<()> {
    (|| fs::rename(from, to))
        .retry(
            ConstantBuilder::default()
                .with_delay(Duration::from_millis(100))
                .with_max_times(20),
        )
        .await
}

pub async fn remove_files(game: &GameDir, item_id: u32) {
    let item_dir = game.item(item_id);
    if item_dir.exists() {
        fs::remove_dir_all(&item_dir).await.ok();
    }
}

/// The one jar of an installed add-on revision.
pub async fn addon_jar(revision_dir: &Path) -> Result<PathBuf> {
    let installation = installation_dir(revision_dir)
        .await
        .with_context(|| format!("No installed files in {}", revision_dir.display()))?;

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
    match jars.len() {
        1 => Ok(jars.remove(0)),
        count => anyhow::bail!(
            "Expected exactly one jar in {}, found {count}",
            installation.display()
        ),
    }
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
pub mod tests {
    use super::*;

    pub fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("liquidlauncher-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
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
