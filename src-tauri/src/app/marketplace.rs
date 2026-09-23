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
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tracing::{info, warn};

use crate::app::client_api::{Build, Client, LiquidBounceRange, MarketplaceRevision};
use crate::minecraft::progress::{ProgressReceiver, ProgressUpdate};
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

/// A build's date is its commit date, and LiquidBounce #9122 brought add-ons at this one. The
/// version cannot tell: 0.40.1 builds exist on both sides of it.
fn supports_addons(build: &Build) -> bool {
    build.date >= Utc.with_ymd_and_hms(2026, 9, 14, 17, 38, 44).unwrap()
}

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
    let addons: Vec<_> = read_subscriptions(data, &build.branch)
        .await?
        .into_iter()
        .filter(|item| item.item_type == MarketplaceItemType::Addon)
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

    let mods = mods_dir(data, &build.branch);
    fs::create_dir_all(&mods).await?;
    let mut remembered = read_staged(data).await;
    let unchanged = remembered.clone();

    for item in &addons {
        let Some((staged, jar)) = resolve(client, data, build, item, &remembered, progress).await
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

/// Every revision that fits the build, newest first.
async fn compatible_revisions(
    client: &Client,
    build: &Build,
    item_id: u32,
) -> Result<Vec<MarketplaceRevision>> {
    let mut revisions = vec![];
    for page in 1.. {
        let response = client
            .marketplace_compatible_revisions(item_id, &build.mc_version, &build.lb_version, page)
            .await?;
        revisions.extend(response.items);
        if page >= response.pagination.pages {
            break;
        }
    }
    Ok(revisions)
}

/// The revisions that fit the build and the newest one of all.
async fn revisions(
    client: &Client,
    build: &Build,
    item_id: u32,
) -> Result<(Vec<MarketplaceRevision>, Option<MarketplaceRevision>)> {
    let (compatible, newest) = tokio::try_join!(
        compatible_revisions(client, build, item_id),
        client.marketplace_revisions(item_id),
    )?;
    Ok((compatible, newest.items.into_iter().next()))
}

/// Picks the revision of `item` to stage, or without the marketplace the one staged for the
/// build's versions last time. Makes it the installed one and finds its jar, or logs why there is
/// none. Other installed files stay, another build may take them.
async fn resolve(
    client: &Client,
    data: &Path,
    build: &Build,
    item: &SubscribedItem,
    remembered: &[Staged],
    progress: &impl ProgressReceiver,
) -> Option<(Staged, PathBuf)> {
    let item_dir = item_dir(data, &build.branch, item.id);
    let installed = installed_revision(&item_dir).await;

    let for_build = |revision: &MarketplaceRevision| Staged {
        item: item.id,
        liquidbounce: build.lb_version.clone(),
        minecraft: build.mc_version.clone(),
        revision: revision.id,
        version: revision.version.clone(),
    };
    let (target, fallback) = match revisions(client, build, item.id).await {
        Ok((compatible, newest)) => {
            let (target, fallback) = pick(
                item,
                &build.lb_version,
                &compatible,
                newest.as_ref(),
                installed,
                progress,
            )?;
            (for_build(target), fallback.map(for_build))
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
    let url = client.marketplace_download_url(item.id, target.revision);
    let chosen = match install(data, &build.branch, item.id, target.revision, &url).await {
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

/// The newest revision that fits, and the installed one if it may stand in when installing that
/// fails.
fn pick<'a>(
    item: &SubscribedItem,
    liquidbounce: &str,
    compatible: &'a [MarketplaceRevision],
    newest: Option<&MarketplaceRevision>,
    installed: Option<u32>,
    progress: &impl ProgressReceiver,
) -> Option<(&'a MarketplaceRevision, Option<&'a MarketplaceRevision>)> {
    let Some(target) = compatible.first() else {
        progress.log(&no_version(&item.name, liquidbounce));
        return None;
    };
    if let Some(newest) = newest.filter(|newest| newest.id != target.id) {
        progress.log(&held_back(&item.name, target, newest));
    }

    let fallback = compatible
        .iter()
        .find(|revision| Some(revision.id) == installed);
    Some((target, fallback))
}

/// Versions read `v1.0.0` whether or not the marketplace names them so.
fn display_version(version: &str) -> String {
    if version.starts_with(|c: char| c.is_ascii_digit()) {
        format!("v{version}")
    } else {
        version.to_owned()
    }
}

/// `LiquidBounce v0.39.0 - 0.40.0`, the builds a revision fits.
fn range_label(range: &LiquidBounceRange) -> String {
    if range.min == range.max {
        format!("LiquidBounce v{}", range.min)
    } else {
        format!("LiquidBounce v{} - {}", range.min, range.max)
    }
}

/// `v1.1.0 needs LiquidBounce v0.41.0`, unless no build fits `revision`.
fn needs(revision: &MarketplaceRevision) -> Option<String> {
    let range = revision.liquidbounce.as_ref()?;
    Some(format!(
        "{} needs {}",
        display_version(&revision.version),
        range_label(range)
    ))
}

fn held_back(name: &str, target: &MarketplaceRevision, newest: &MarketplaceRevision) -> String {
    let stays = format!("{name} stays on {}", display_version(&target.version));
    match needs(newest) {
        Some(needs) => format!("{stays}: {needs}."),
        None => format!("{stays}."),
    }
}

fn no_version(name: &str, liquidbounce: &str) -> String {
    format!(
        "{name} is not for LiquidBounce {}.",
        display_version(liquidbounce)
    )
}

/// The one jar of an installed add-on revision.
async fn addon_jar(revision_dir: &Path) -> Result<PathBuf> {
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

    #[test]
    fn picks_the_newest_revision_that_fits() {
        struct Log(std::cell::RefCell<Vec<String>>);
        impl ProgressReceiver for Log {
            fn progress_update(&self, _: ProgressUpdate) {}
            fn log(&self, msg: &str) {
                self.0.borrow_mut().push(msg.to_owned());
            }
        }

        let revision = |id, range: Option<(&str, &str)>| MarketplaceRevision {
            id,
            version: format!("1.{id}.0"),
            changelog: None,
            liquidbounce: range.map(|(min, max)| LiquidBounceRange {
                min: min.to_owned(),
                max: max.to_owned(),
            }),
        };
        let item = SubscribedItem {
            name: "Extras".to_owned(),
            id: 781,
            item_type: MarketplaceItemType::Addon,
        };
        let log = Log(Default::default());
        let pick = |compatible: &[MarketplaceRevision], newest, installed| {
            pick(&item, "0.40.1", compatible, newest, installed, &log)
                .map(|(target, fallback)| (target.id, fallback.map(|fallback| fallback.id)))
        };

        let fitting = [
            revision(2, Some(("0.40.0", "0.40.1"))),
            revision(1, Some(("0.40.1", "0.40.1"))),
        ];
        let newer = revision(3, Some(("0.41.0", "0.41.2")));
        let unfit = revision(4, None);
        assert_eq!(
            pick(&fitting, Some(&fitting[0]), Some(1)),
            Some((2, Some(1)))
        );
        assert_eq!(pick(&fitting, Some(&newer), Some(3)), Some((2, None)));
        assert_eq!(pick(&fitting, Some(&unfit), None), Some((2, None)));
        assert_eq!(pick(&[], Some(&newer), Some(3)), None);
        assert_eq!(pick(&[], None, None), None);

        assert_eq!(
            log.0.into_inner(),
            [
                "Extras stays on v1.2.0: v1.3.0 needs LiquidBounce v0.41.0 - 0.41.2.",
                "Extras stays on v1.2.0.",
                "Extras is not for LiquidBounce v0.40.1.",
                "Extras is not for LiquidBounce v0.40.1.",
            ]
        );
    }

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

    #[test]
    fn reads_the_builds_a_revision_fits() {
        let revisions: Vec<MarketplaceRevision> = serde_json::from_value(json!([
            {
                "id": 19,
                "version": "1.0.1",
                "created_at": "2026-09-20T10:00:00",
                "liquidbounce": { "min": "0.40.0", "max": "0.40.1" },
            },
            {
                "id": 5,
                "version": "1.1.0",
                "liquidbounce": { "min": "0.41.0", "max": "0.41.0" },
            },
            { "id": 4, "version": "1.0.0", "liquidbounce": null },
            { "id": 3, "version": "1.0.0" },
        ]))
        .unwrap();
        let ranges: Vec<_> = revisions
            .iter()
            .map(|revision| revision.liquidbounce.as_ref().map(range_label))
            .collect();
        assert_eq!(
            ranges,
            [
                Some("LiquidBounce v0.40.0 - 0.40.1".to_owned()),
                Some("LiquidBounce v0.41.0".to_owned()),
                None,
                None
            ]
        );
    }

    #[test]
    fn only_builds_with_the_addon_system_get_addons() {
        // Release 0.40.0, the last nightly before #9122, #9122 itself, the first nightly after it.
        for (build_id, lb_version, date, expected) in [
            (16941, "0.40.0", "2026-08-21T03:16:52Z", false),
            (17221, "0.40.1", "2026-09-14T12:35:25Z", false),
            (0, "0.40.1", "2026-09-14T17:38:44Z", true),
            (17223, "0.40.1", "2026-09-14T18:08:10Z", true),
        ] {
            let build: Build = serde_json::from_value(json!({
                "build_id": build_id,
                "commit_id": "",
                "branch": "nextgen",
                "subsystem": "fabric",
                "lb_version": lb_version,
                "mc_version": "26.2",
                "release": build_id == 16941,
                "date": date,
                "message": "",
                "url": "",
                "jre_version": 25,
                "jre_distribution": "temurin",
                "fabric_api_version": "0.153.0+26.2",
                "fabric_loader_version": "0.19.3",
                "kotlin_version": "2.4.0",
                "kotlin_mod_version": "1.13.12+kotlin.2.4.0",
            }))
            .unwrap();
            assert_eq!(supports_addons(&build), expected, "build {build_id}");
        }
    }
}
