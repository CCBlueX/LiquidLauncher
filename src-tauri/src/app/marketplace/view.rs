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

//! What the launcher shows about add-ons, themes and scripts, for the build selected to launch.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use anyhow::{bail, Result};
use futures::future::join_all;
use regex::Regex;
use serde::Serialize;
use tracing::warn;

use super::api::{self, Revision};
use super::install;
use super::revisions::{display_version, range_label, supports_addons};
use super::{queued, subscriptions, GameDir, ItemType, Queued, SubscribedItem};
use crate::app::builds::Selection;
use crate::app::client_api::{Build, Client};
use crate::utils::{error_line, short_date};

const LISTED: u32 = 50;
const VERSIONS: u32 = 5;

/// The one notice the library shows above its lists, the first that applies.
#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Notice {
    Idle,
    Checking,
    Offline {
        error: String,
    },
    /// Edits made while the game runs wait for it to exit.
    Restart {
        changes: usize,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    id: u32,
    name: String,
    /// What is installed. Only a theme's files name it, the rest waits for the marketplace.
    version: Option<String>,
    /// The build's LiquidBounce version, for an add-on that has nothing for it.
    not_for: Option<String>,
    /// Removed while the game runs, until it exits.
    removed: bool,
    /// The installed items that stop working without it, once the marketplace was asked.
    needed_by: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    minecraft: Option<String>,
    liquidbounce: Option<String>,
    selection: Option<Selection>,
    notice: Notice,
    addons: Vec<LibraryItem>,
    themes: Vec<LibraryItem>,
    scripts: Vec<LibraryItem>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BrowseFit {
    Fits {
        version: String,
        liquidbounce: Option<String>,
    },
    NoVersion,
    /// The marketplace could not be asked.
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseItem {
    id: u32,
    name: String,
    summary: String,
    downloads: u32,
    date: Option<String>,
    fit: Option<BrowseFit>,
    subscribed: bool,
    /// LiquidBounce refuses an add-on that has no revision for the build as well.
    installable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Browse {
    liquidbounce: String,
    items: Vec<BrowseItem>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Tag {
    Installed,
    NotFor { liquidbounce: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionRow {
    label: String,
    liquidbounce: Option<String>,
    date: Option<String>,
    tag: Option<Tag>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    id: u32,
    name: String,
    #[serde(rename = "type")]
    item_type: ItemType,
    author: String,
    downloads: u32,
    summary: String,
    screenshots: Vec<Screenshot>,
    /// Removing is offered, otherwise installing.
    subscribed: bool,
    can_install: bool,
    versions: Vec<VersionRow>,
    needed_by: Vec<NeededBy>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeededBy {
    name: String,
    #[serde(rename = "type")]
    item_type: ItemType,
    author: String,
}

/// What the marketplace tells about an installed add-on or script.
struct Checked {
    /// A revision fits the build. Scripts fit any.
    fits: bool,
    /// The installed revision's.
    version: Option<String>,
}

/// Asks the marketplace about an add-on or script. Themes are not asked.
async fn check(
    client: &Client,
    build: &Build,
    item: &SubscribedItem,
    installed: Option<u32>,
) -> Option<Result<Checked>> {
    let fitting = match item.item_type {
        ItemType::Addon if supports_addons(build) => {
            match api::revisions_for(client, item.id, build).await {
                Ok(fitting) => Some(fitting),
                Err(error) => return Some(Err(error)),
            }
        }
        ItemType::Addon => Some(vec![]),
        ItemType::Script => None,
        _ => return None,
    };

    let fits = fitting.as_ref().is_none_or(|fitting| !fitting.is_empty());
    let version = installed_version(client, item.id, installed, fitting.iter().flatten()).await;
    Some(Ok(Checked { fits, version }))
}

/// The version of the installed revision, from the revisions fetched already or the marketplace.
/// Only telling, so a failure leaves it out.
async fn installed_version<'a>(
    client: &Client,
    item_id: u32,
    installed: Option<u32>,
    fetched: impl IntoIterator<Item = &'a Revision>,
) -> Option<String> {
    let installed = installed?;
    if let Some(revision) = fetched
        .into_iter()
        .find(|revision| revision.id == installed)
    {
        return Some(revision.version.clone());
    }

    api::revision(client, item_id, installed)
        .await
        .inspect_err(|error| warn!("Failed to look up revision {installed}: {:?}", error))
        .ok()
        .map(|revision| revision.version)
}

/// The build's LiquidBounce version for an add-on that has nothing for the build. `fits` is `None`
/// while the marketplace was not asked.
fn not_for(supports: bool, fits: Option<bool>, liquidbounce: &str) -> Option<String> {
    (!supports || fits == Some(false)).then(|| display_version(liquidbounce))
}

/// Marks the installed revision, and those that do not fit the build. `fitting` is `None` for
/// themes and scripts.
fn tag(
    revision: u32,
    installed: Option<u32>,
    fitting: Option<&HashSet<u32>>,
    liquidbounce: &str,
) -> Option<Tag> {
    if installed == Some(revision) {
        return Some(Tag::Installed);
    }
    fitting
        .filter(|fitting| !fitting.contains(&revision))
        .map(|_| Tag::NotFor {
            liquidbounce: display_version(liquidbounce),
        })
}

/// Add-ons, themes and scripts subscribed, with anything still waiting for the game to exit.
async fn library_items(game: &GameDir, queued: &Queued) -> Result<Vec<SubscribedItem>> {
    let mut items = subscriptions::read(game).await?;
    for adding in queued.adding() {
        if !items.iter().any(|item| item.id == adding.id) {
            items.push(adding.clone());
        }
    }
    items.retain(|item| {
        matches!(
            item.item_type,
            ItemType::Addon | ItemType::Theme | ItemType::Script
        )
    });
    Ok(items)
}

fn is_installable(item_type: ItemType) -> bool {
    matches!(item_type, ItemType::Addon | ItemType::Script)
}

/// The add-ons and scripts each add-on and script of `items` needs beside it, by item id.
async fn needs(client: &Client, items: &[SubscribedItem]) -> Result<HashMap<u32, Vec<u32>>> {
    let needs = items
        .iter()
        .filter(|item| is_installable(item.item_type))
        .map(|item| async {
            let needed = api::dependencies(client, item.id)
                .await?
                .into_iter()
                .filter(|needed| is_installable(needed.item_type))
                .map(|needed| needed.id)
                .collect();
            anyhow::Ok((item.id, needed))
        });
    join_all(needs).await.into_iter().collect()
}

/// The items of `items` that stay and need `item_id`.
fn dependents<'a>(
    items: &'a [SubscribedItem],
    needs: &'a HashMap<u32, Vec<u32>>,
    queued: &'a Queued,
    item_id: u32,
) -> impl Iterator<Item = &'a SubscribedItem> {
    items.iter().filter(move |other| {
        !queued.removing(other.id)
            && needs
                .get(&other.id)
                .is_some_and(|needed| needed.contains(&item_id))
    })
}

/// Whether and how the library asks the marketplace.
pub enum Remote<'a> {
    Skip,
    Check(&'a Client),
    /// The build could not be resolved.
    Failed(String),
}

/// The build the library is for.
pub struct Selected<'a> {
    pub build: Option<&'a Build>,
    /// "Latest" is selected rather than this build.
    pub latest: bool,
}

pub async fn library(
    game: &GameDir,
    selected: &Selected<'_>,
    remote: Remote<'_>,
) -> Result<Library> {
    let build = selected.build;
    let queued = queued();
    let items = library_items(game, &queued).await?;
    let client = match remote {
        Remote::Check(client) => Some(client),
        _ => None,
    };

    let checks = items.iter().map(|item| async {
        let installed = install::installed(game, item).await;
        let checked = match (client, build) {
            (Some(client), Some(build)) => {
                let revision = installed.as_ref().map(|installed| installed.revision);
                check(client, build, item, revision).await
            }
            _ => None,
        };
        (installed, checked)
    });
    let (checked, needs) = tokio::join!(join_all(checks), async {
        match client {
            Some(client) => Some(needs(client, &items).await),
            None => None,
        }
    });

    let mut offline = match &remote {
        Remote::Failed(error) => Some(error.clone()),
        _ => None,
    };
    let mut failed = |error: anyhow::Error| {
        offline.get_or_insert_with(|| {
            format!("unable to check the marketplace: {}", error_line(&error))
        });
    };

    let needs = match needs {
        Some(Ok(needs)) => needs,
        Some(Err(error)) => {
            warn!("Failed to look up marketplace dependencies: {:?}", error);
            failed(error);
            HashMap::new()
        }
        None => HashMap::new(),
    };

    let supports = build.is_none_or(supports_addons);
    let liquidbounce = build.map_or("", |build| &build.lb_version);
    let mut library = Library {
        minecraft: build.map(|build| build.mc_version.clone()),
        liquidbounce: build.map(|build| build.lb_version.clone()),
        selection: build.map(|build| Selection::of(build, selected.latest)),
        notice: Notice::Checking,
        addons: vec![],
        themes: vec![],
        scripts: vec![],
    };

    for (item, (installed, checked)) in items.iter().zip(checked) {
        let checked = match checked {
            Some(Ok(checked)) => Some(checked),
            Some(Err(error)) => {
                warn!(
                    "Failed to check marketplace item '{}': {:?}",
                    item.name, error
                );
                failed(error);
                None
            }
            None => None,
        };

        let fits = checked.as_ref().map(|checked| checked.fits);
        let version = match checked {
            Some(checked) => checked.version,
            None => installed.and_then(|installed| installed.version),
        };
        let row = LibraryItem {
            id: item.id,
            name: item.name.clone(),
            version: version.map(|version| display_version(&version)),
            not_for: (item.item_type == ItemType::Addon)
                .then(|| not_for(supports, fits, liquidbounce))
                .flatten(),
            removed: queued.removing(item.id),
            needed_by: dependents(&items, &needs, &queued, item.id)
                .map(|other| other.name.clone())
                .collect(),
        };
        match item.item_type {
            ItemType::Addon => library.addons.push(row),
            ItemType::Script => library.scripts.push(row),
            _ => library.themes.push(row),
        }
    }

    if !matches!(remote, Remote::Skip) {
        let changes = queued.len();
        library.notice = match offline {
            Some(error) => Notice::Offline { error },
            None if changes > 0 => Notice::Restart { changes },
            None => Notice::Idle,
        };
    }
    Ok(library)
}

pub async fn browse(
    client: &Client,
    game: &GameDir,
    build: &Build,
    item_type: ItemType,
    query: Option<&str>,
) -> Result<Browse> {
    let type_name = match item_type {
        ItemType::Addon => "Addon",
        ItemType::Theme => "Theme",
        ItemType::Script => "Script",
        other => bail!("{other:?} is not listed here"),
    };
    let listed = api::items(client, type_name, query, LISTED).await?;

    let queued = queued();
    let subscribed: HashSet<_> = library_items(game, &queued)
        .await?
        .iter()
        .map(|item| item.id)
        .filter(|id| !queued.removing(*id))
        .collect();
    let supports = supports_addons(build);

    let fits = listed.iter().map(|item| async {
        if item.item_type != ItemType::Addon {
            return None;
        }
        if !supports {
            return Some(BrowseFit::NoVersion);
        }

        Some(match api::revisions_for(client, item.id, build).await {
            Ok(fitting) => match fitting.first() {
                Some(target) => BrowseFit::Fits {
                    version: display_version(&target.version),
                    liquidbounce: target.liquidbounce.as_ref().map(range_label),
                },
                None => BrowseFit::NoVersion,
            },
            Err(error) => {
                warn!("Failed to check add-on '{}': {:?}", item.name, error);
                BrowseFit::Unknown
            }
        })
    });
    let fits = join_all(fits).await;

    let items = listed
        .into_iter()
        .zip(fits)
        .map(|(item, fit)| BrowseItem {
            id: item.id,
            subscribed: subscribed.contains(&item.id),
            installable: !matches!(fit, Some(BrowseFit::NoVersion)),
            summary: summary(&item.description),
            downloads: item.downloads,
            date: item.updated_at.map(short_date),
            fit,
            name: item.name,
        })
        .collect();

    Ok(Browse {
        liquidbounce: display_version(&build.lb_version),
        items,
    })
}

pub async fn detail(
    client: &Client,
    game: &GameDir,
    build: &Build,
    item_id: u32,
) -> Result<Detail> {
    let (item, revisions) = tokio::try_join!(
        api::item(client, item_id),
        api::revisions(client, item_id, VERSIONS),
    )?;
    let is_addon = match item.item_type {
        ItemType::Addon => true,
        ItemType::Theme | ItemType::Script => false,
        other => bail!("{other:?} is not shown here"),
    };
    // Theme versions are builds, so their dates name them.
    let is_theme = item.item_type == ItemType::Theme;

    let queued = queued();
    let subscribed_item = SubscribedItem {
        name: item.name.clone(),
        id: item.id,
        item_type: item.item_type,
    };
    let items = library_items(game, &queued).await?;
    let listed = items.iter().any(|subscribed| subscribed.id == item.id);
    let needed_by = needed_by(client, &items, &queued, item.id).await;
    let installed = install::installed(game, &subscribed_item)
        .await
        .map(|installed| installed.revision)
        .filter(|_| listed);

    let fitting: Option<HashSet<_>> = if !is_addon {
        None
    } else if supports_addons(build) {
        let fitting = api::revisions_for(client, item.id, build).await?;
        Some(fitting.iter().map(|revision| revision.id).collect())
    } else {
        Some(HashSet::new())
    };

    let can_install = match &fitting {
        Some(fitting) => !fitting.is_empty(),
        None => !revisions.is_empty(),
    };
    let versions = revisions
        .iter()
        .map(|revision| {
            let date = revision.created_at.map(short_date);
            VersionRow {
                label: match (is_theme, &date) {
                    (true, Some(date)) => date.clone(),
                    _ => display_version(&revision.version),
                },
                liquidbounce: revision.liquidbounce.as_ref().map(range_label),
                date: date.filter(|_| !is_theme),
                tag: tag(revision.id, installed, fitting.as_ref(), &build.lb_version),
            }
        })
        .collect();

    Ok(Detail {
        id: item.id,
        item_type: item.item_type,
        author: item.author,
        downloads: item.downloads,
        summary: summary(&item.description),
        screenshots: screenshots(&item.description),
        subscribed: listed && !queued.removing(item.id),
        can_install,
        versions,
        needed_by,
        name: item.name,
    })
}

/// The installed items that need `item_id`. Only telling, so a failure leaves it empty.
async fn needed_by(
    client: &Client,
    items: &[SubscribedItem],
    queued: &Queued,
    item_id: u32,
) -> Vec<NeededBy> {
    let others: Vec<_> = items
        .iter()
        .filter(|other| other.id != item_id && !queued.removing(other.id))
        .cloned()
        .collect();
    let needs = match needs(client, &others).await {
        Ok(needs) => needs,
        Err(error) => {
            warn!("Failed to look up marketplace dependencies: {:?}", error);
            return vec![];
        }
    };

    join_all(
        dependents(&others, &needs, queued, item_id).map(|other| async move {
            let author = match api::item(client, other.id).await {
                Ok(dependent) => dependent.author,
                Err(error) => {
                    warn!("Failed to look up '{}': {:?}", other.name, error);
                    String::new()
                }
            };
            NeededBy {
                name: other.name.clone(),
                item_type: other.item_type,
                author,
            }
        }),
    )
    .await
}

/// The first paragraph of a markdown description, as plain text.
fn summary(description: &str) -> String {
    static INLINE: LazyLock<[(Regex, &str); 5]> = LazyLock::new(|| {
        [
            (Regex::new(r"!\[[^\]]*\]\([^)]*\)").unwrap(), ""),
            (Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap(), "$1"),
            (Regex::new(r"<[^>]+>").unwrap(), ""),
            (Regex::new(r"(\*\*|__)(.+?)(\*\*|__)").unwrap(), "$2"),
            (Regex::new(r"[*`]([^*`]+)[*`]").unwrap(), "$1"),
        ]
    });
    const BLOCKS: [&str; 9] = ["#", "!", "<", "|", ">", "```", "---", "- ", "* "];

    let description = description.replace("\r\n", "\n");
    for paragraph in description.split("\n\n") {
        let paragraph = paragraph.trim();
        if BLOCKS.iter().any(|block| paragraph.starts_with(block)) {
            continue;
        }

        let mut text = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
        for (pattern, replacement) in INLINE.iter() {
            text = pattern.replace_all(&text, *replacement).into_owned();
        }
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Screenshot {
    url: String,
    caption: String,
}

/// The images of a markdown description, with their alternative texts as captions.
fn screenshots(description: &str) -> Vec<Screenshot> {
    static IMAGE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"!\[([^\]]*)\]\(\s*<?(https?://[^)\s>]+)|<img[^>]*>"#).unwrap()
    });
    static SRC: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"\ssrc="(https?://[^"]+)""#).unwrap());
    static ALT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\salt="([^"]*)""#).unwrap());

    IMAGE
        .captures_iter(description)
        .filter_map(|image| {
            let (url, caption) = match image.get(2) {
                Some(url) => (url.as_str(), &image[1]),
                None => {
                    let tag = &image[0];
                    let alt = ALT.captures(tag);
                    let url = SRC.captures(tag)?.get(1)?.as_str();
                    (
                        url,
                        alt.and_then(|alt| alt.get(1))
                            .map_or("", |alt| alt.as_str()),
                    )
                }
            };
            Some(Screenshot {
                url: url.to_owned(),
                caption: caption.to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_an_addon_that_has_nothing_for_the_build() {
        assert_eq!(not_for(true, None, "0.40.1"), None);
        assert_eq!(not_for(true, Some(true), "0.40.1"), None);
        assert_eq!(
            not_for(true, Some(false), "0.40.1").as_deref(),
            Some("v0.40.1")
        );
        assert_eq!(not_for(false, None, "0.40.0").as_deref(), Some("v0.40.0"));
    }

    #[test]
    fn tags_the_installed_revision_and_those_not_for_the_build() {
        let fitting = HashSet::from([1, 2]);
        let not_for = || {
            Some(Tag::NotFor {
                liquidbounce: "v0.40.1".to_owned(),
            })
        };
        assert_eq!(
            tag(2, Some(2), Some(&fitting), "0.40.1"),
            Some(Tag::Installed)
        );
        assert_eq!(tag(1, Some(2), Some(&fitting), "0.40.1"), None);
        assert_eq!(tag(3, Some(2), Some(&fitting), "0.40.1"), not_for());
        assert_eq!(
            tag(3, Some(3), Some(&fitting), "0.40.1"),
            Some(Tag::Installed)
        );
        assert_eq!(tag(3, None, None, "0.40.1"), None);
        assert_eq!(tag(1, None, Some(&HashSet::new()), "0.40.1"), not_for());
    }

    #[test]
    fn reads_a_description() {
        let extras = "Modules that [LiquidBounce](https://github.com/CCBlueX/LiquidBounce) does not ship. They show up in the ClickGUI under their own **Extras** category.\n\n![Extras in the ClickGUI](https://ccbluex-api-public.s3.de.io.cloud.ovh.net/marketplace/screenshots/781/LzCa1tNKl1LT.png)\n\n## Modules";
        assert_eq!(
            summary(extras),
            "Modules that LiquidBounce does not ship. They show up in the ClickGUI under their own Extras category."
        );
        assert_eq!(
            screenshots(extras),
            [Screenshot {
                url: "https://ccbluex-api-public.s3.de.io.cloud.ovh.net/marketplace/screenshots/781/LzCa1tNKl1LT.png".to_owned(),
                caption: "Extras in the ClickGUI".to_owned(),
            }]
        );
        assert_eq!(
            screenshots(
                r#"<p><img alt="HUD" src="https://example.com/hud.png"/> <img src="http://example.com/a.png"></p>"#
            ),
            [
                Screenshot {
                    url: "https://example.com/hud.png".to_owned(),
                    caption: "HUD".to_owned()
                },
                Screenshot {
                    url: "http://example.com/a.png".to_owned(),
                    caption: String::new()
                },
            ]
        );

        let heading = "# ScriptAPI\r\n\r\nThe `JavaScript` Script API,\r\npackaged as an *add-on*.";
        assert_eq!(
            summary(heading),
            "The JavaScript Script API, packaged as an add-on."
        );
        assert!(screenshots(heading).is_empty());
    }
}
