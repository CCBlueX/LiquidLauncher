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

//! What the Marketplace tab shows about themes and add-ons, for the build selected to launch.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{bail, Result};
use chrono::{Datelike, NaiveDateTime, Utc};
use futures::future::join_all;
use regex::Regex;
use serde::Serialize;
use tracing::warn;

use super::{
    compatible_revisions, display_version, no_version, pending, range_label, read_installed,
    read_subscriptions, revisions, supports_addons, Installed, MarketplaceItemType, Pending,
    SubscribedItem,
};
use crate::app::client_api::{Build, Client, MarketplaceRevision};

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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    minecraft: Option<String>,
    liquidbounce: Option<String>,
    notice: Notice,
    addons: Vec<LibraryItem>,
    themes: Vec<LibraryItem>,
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
    item_type: MarketplaceItemType,
    author: String,
    downloads: u32,
    summary: String,
    preview: Option<String>,
    /// Removing is offered, otherwise installing.
    subscribed: bool,
    can_install: bool,
    versions: Vec<VersionRow>,
}

/// What the marketplace tells about an installed add-on.
struct Checked {
    /// A revision fits the build.
    fits: bool,
    /// The installed revision's.
    version: Option<String>,
}

/// Whether an add-on has a revision for the build, and the version of the installed one.
async fn check_addon(
    client: &Client,
    build: &Build,
    supports: bool,
    item_id: u32,
    installed: Option<u32>,
) -> Result<Checked> {
    if !supports {
        let version = installed_version(client, item_id, installed, None).await;
        return Ok(Checked {
            fits: false,
            version,
        });
    }

    let compatible = compatible_revisions(client, build, item_id).await?;
    let version = installed_version(client, item_id, installed, &compatible).await;
    Ok(Checked {
        fits: !compatible.is_empty(),
        version,
    })
}

/// The version of the installed revision, from the revisions fetched already or the marketplace.
/// Only telling, so a failure leaves it out.
async fn installed_version<'a>(
    client: &Client,
    item_id: u32,
    installed: Option<u32>,
    fetched: impl IntoIterator<Item = &'a MarketplaceRevision>,
) -> Option<String> {
    let installed = installed?;
    if let Some(revision) = fetched
        .into_iter()
        .find(|revision| revision.id == installed)
    {
        return Some(revision.version.clone());
    }

    client
        .marketplace_revision(item_id, installed)
        .await
        .inspect_err(|error| warn!("Failed to look up revision {installed}: {:?}", error))
        .ok()
        .map(|revision| revision.version)
}

/// The revision installing an add-on for the build takes.
pub async fn install_target(client: &Client, build: &Build, item: &SubscribedItem) -> Result<u32> {
    let (compatible, newest) = revisions(client, build, item.id).await?;
    if let Some(target) = compatible.first() {
        return Ok(target.id);
    }
    if newest.is_none() {
        bail!("{} has nothing published yet.", item.name);
    }
    bail!("{}", no_version(&item.name, &build.lb_version))
}

/// What is known about one item without asking the marketplace.
struct Facts {
    installed: Option<Installed>,
    removing: bool,
}

impl Facts {
    async fn read(data: &Path, branch: &str, item: &SubscribedItem, pending: &Pending) -> Self {
        Facts {
            installed: read_installed(data, branch, item).await,
            removing: pending.removing.contains(&item.id),
        }
    }

    fn installed_revision(&self) -> Option<u32> {
        self.installed.as_ref().map(|installed| installed.revision)
    }
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

fn short_date(date: NaiveDateTime) -> String {
    if date.year() == Utc::now().year() {
        date.format("%b %-d").to_string()
    } else {
        date.format("%b %-d, %Y").to_string()
    }
}

/// The error as one line, without the request URL.
pub fn describe(error: &anyhow::Error) -> String {
    static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" for url \([^)]*\)").unwrap());
    URL.replace_all(&format!("{error:#}"), "").into_owned()
}

/// Themes and add-ons subscribed, with anything still waiting for the game to exit.
async fn library_items(
    data: &Path,
    branch: &str,
    pending: &Pending,
) -> Result<Vec<SubscribedItem>> {
    let mut items = read_subscriptions(data, branch).await?;
    for queued in &pending.subscribing {
        if !items.iter().any(|item| item.id == queued.id) {
            items.push(queued.clone());
        }
    }
    items.retain(|item| {
        matches!(
            item.item_type,
            MarketplaceItemType::Addon | MarketplaceItemType::Theme
        )
    });
    Ok(items)
}

/// Whether and how the library asks the marketplace.
pub enum Remote<'a> {
    Skip,
    Check(&'a Client),
    /// The build could not be resolved.
    Failed(String),
}

pub async fn library(
    data: &Path,
    branch: &str,
    build: Option<&Build>,
    remote: Remote<'_>,
) -> Result<Library> {
    let pending = pending();
    let items = library_items(data, branch, &pending).await?;
    let supports = build.is_none_or(supports_addons);
    let liquidbounce = build.map_or("", |build| &build.lb_version);

    let checks = items.iter().map(|item| async {
        let facts = Facts::read(data, branch, item, &pending).await;
        let Remote::Check(client) = remote else {
            return (facts, None);
        };
        let Some(build) = build else {
            return (facts, None);
        };
        if item.item_type != MarketplaceItemType::Addon {
            return (facts, None);
        }

        let installed = facts.installed_revision();
        let checked = check_addon(client, build, supports, item.id, installed).await;
        (facts, Some(checked))
    });
    let checked = join_all(checks).await;

    let mut offline = match &remote {
        Remote::Failed(error) => Some(error.clone()),
        _ => None,
    };
    let mut library = Library {
        minecraft: build.map(|build| build.mc_version.clone()),
        liquidbounce: build.map(|build| build.lb_version.clone()),
        notice: Notice::Checking,
        addons: vec![],
        themes: vec![],
    };

    for (item, (facts, checked)) in items.iter().zip(checked) {
        let checked = match checked {
            Some(Ok(checked)) => Some(checked),
            Some(Err(error)) => {
                warn!(
                    "Failed to check marketplace item '{}': {:?}",
                    item.name, error
                );
                offline.get_or_insert_with(|| {
                    format!("unable to check the marketplace: {}", describe(&error))
                });
                None
            }
            None => None,
        };

        let is_addon = item.item_type == MarketplaceItemType::Addon;
        let fits = checked.as_ref().map(|checked| checked.fits);
        let version = match checked {
            Some(checked) => checked.version,
            None => facts.installed.and_then(|installed| installed.version),
        };
        let row = LibraryItem {
            id: item.id,
            name: item.name.clone(),
            version: version.map(|version| display_version(&version)),
            not_for: is_addon
                .then(|| not_for(supports, fits, liquidbounce))
                .flatten(),
            removed: facts.removing,
        };
        if is_addon {
            library.addons.push(row);
        } else {
            library.themes.push(row);
        }
    }

    if !matches!(remote, Remote::Skip) {
        let changes = pending.subscribing.len() + pending.removing.len();
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
    data: &Path,
    branch: &str,
    build: &Build,
    item_type: MarketplaceItemType,
    query: Option<&str>,
) -> Result<Browse> {
    let type_name = match item_type {
        MarketplaceItemType::Addon => "Addon",
        MarketplaceItemType::Theme => "Theme",
        other => bail!("{other:?} is not listed here"),
    };
    let listed = client.marketplace_items(LISTED, query, type_name).await?;

    let pending = pending();
    let subscribed: HashSet<_> = library_items(data, branch, &pending)
        .await?
        .iter()
        .map(|item| item.id)
        .filter(|id| !pending.removing.contains(id))
        .collect();
    let supports = supports_addons(build);

    let fits = listed.items.iter().map(|item| async {
        if item.item_type != MarketplaceItemType::Addon {
            return None;
        }
        if !supports {
            return Some(BrowseFit::NoVersion);
        }

        Some(match compatible_revisions(client, build, item.id).await {
            Ok(compatible) => match compatible.first() {
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
        .items
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
    data: &Path,
    branch: &str,
    build: &Build,
    item_id: u32,
) -> Result<Detail> {
    let (item, revisions) = tokio::try_join!(
        client.marketplace_item(item_id),
        client.marketplace_revisions(item_id, VERSIONS),
    )?;
    let revisions = revisions.items;
    let is_addon = match item.item_type {
        MarketplaceItemType::Addon => true,
        MarketplaceItemType::Theme => false,
        other => bail!("{other:?} is not shown here"),
    };

    let pending = pending();
    let subscribed_item = SubscribedItem {
        name: item.name.clone(),
        id: item.id,
        item_type: item.item_type,
    };
    let listed = library_items(data, branch, &pending)
        .await?
        .iter()
        .any(|subscribed| subscribed.id == item.id);
    let facts = Facts::read(data, branch, &subscribed_item, &pending).await;
    let installed = facts.installed_revision().filter(|_| listed);

    let fitting: Option<HashSet<_>> = if !is_addon {
        None
    } else if supports_addons(build) {
        let compatible = compatible_revisions(client, build, item.id).await?;
        Some(compatible.iter().map(|revision| revision.id).collect())
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
                label: match (is_addon, &date) {
                    (false, Some(date)) => date.clone(),
                    _ => display_version(&revision.version),
                },
                liquidbounce: revision.liquidbounce.as_ref().map(range_label),
                date: date.filter(|_| is_addon),
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
        preview: preview(&item.description),
        subscribed: listed && !facts.removing,
        can_install,
        versions,
        name: item.name,
    })
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

/// The first image of a markdown description.
fn preview(description: &str) -> Option<String> {
    static IMAGE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"!\[[^\]]*\]\(\s*<?(https?://[^)\s>]+)|<img[^>]*\ssrc="(https?://[^"]+)""#)
            .unwrap()
    });
    let captures = IMAGE.captures(description)?;
    Some(captures.get(1).or(captures.get(2))?.as_str().to_owned())
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
            preview(extras).as_deref(),
            Some("https://ccbluex-api-public.s3.de.io.cloud.ovh.net/marketplace/screenshots/781/LzCa1tNKl1LT.png")
        );

        let heading = "# ScriptAPI\r\n\r\nThe `JavaScript` Script API,\r\npackaged as an *add-on*.";
        assert_eq!(
            summary(heading),
            "The JavaScript Script API, packaged as an add-on."
        );
        assert_eq!(preview(heading), None);
    }

    #[test]
    fn leaves_the_url_out_of_errors() {
        let error = anyhow::anyhow!(
            "error sending request for url (http://127.0.0.1:1/api/v3/marketplace?page=1)"
        )
        .context("unable to browse marketplace");
        assert_eq!(
            describe(&error),
            "unable to browse marketplace: error sending request"
        );
    }
}
