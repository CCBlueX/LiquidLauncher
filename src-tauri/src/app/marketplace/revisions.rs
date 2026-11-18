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

//! Which revision of an item fits a build, and how the launcher words it.

use anyhow::{bail, Result};
use chrono::{TimeZone, Utc};

use super::api::{self, LiquidBounceRange, Revision};
use super::SubscribedItem;
use crate::app::client_api::{Build, Client};
use crate::minecraft::progress::ProgressReceiver;

/// A build's date is its commit date, and LiquidBounce #9122 brought add-ons at this one. The
/// version cannot tell: 0.40.1 builds exist on both sides of it.
pub fn supports_addons(build: &Build) -> bool {
    build.date >= Utc.with_ymd_and_hms(2026, 9, 14, 17, 38, 44).unwrap()
}

/// The revisions that fit the build and the newest one of all.
pub async fn revisions(
    client: &Client,
    build: &Build,
    item_id: u32,
) -> Result<(Vec<Revision>, Option<Revision>)> {
    let (fitting, newest) = tokio::try_join!(
        api::revisions_for(client, item_id, build),
        api::revisions(client, item_id, 1),
    )?;
    Ok((fitting, newest.into_iter().next()))
}

/// The revision installing an add-on for the build takes.
pub async fn install_target(client: &Client, build: &Build, item: &SubscribedItem) -> Result<u32> {
    let (fitting, newest) = revisions(client, build, item.id).await?;
    if let Some(target) = fitting.first() {
        return Ok(target.id);
    }
    if newest.is_none() {
        bail!("{} has nothing published yet.", item.name);
    }
    bail!("{}", no_version(&item.name, &build.lb_version))
}

/// The newest revision that fits, and the installed one if it may stand in when installing that
/// fails.
pub fn pick<'a>(
    item: &SubscribedItem,
    liquidbounce: &str,
    fitting: &'a [Revision],
    newest: Option<&Revision>,
    installed: Option<u32>,
    progress: &impl ProgressReceiver,
) -> Option<(&'a Revision, Option<&'a Revision>)> {
    let Some(target) = fitting.first() else {
        progress.log(&no_version(&item.name, liquidbounce));
        return None;
    };
    if let Some(newest) = newest.filter(|newest| newest.id != target.id) {
        progress.log(&held_back(&item.name, target, newest));
    }

    let fallback = fitting
        .iter()
        .find(|revision| Some(revision.id) == installed);
    Some((target, fallback))
}

/// Versions read `v1.0.0` whether or not the marketplace names them so.
pub fn display_version(version: &str) -> String {
    if version.starts_with(|c: char| c.is_ascii_digit()) {
        format!("v{version}")
    } else {
        version.to_owned()
    }
}

/// `LiquidBounce v0.39.0 - 0.40.0`, the builds a revision fits.
pub fn range_label(range: &LiquidBounceRange) -> String {
    if range.min == range.max {
        format!("LiquidBounce v{}", range.min)
    } else {
        format!("LiquidBounce v{} - {}", range.min, range.max)
    }
}

pub fn no_version(name: &str, liquidbounce: &str) -> String {
    format!(
        "{name} is not for LiquidBounce {}.",
        display_version(liquidbounce)
    )
}

/// `Extras stays on v1.0.0: v1.1.0 needs LiquidBounce v0.41.0.`
fn held_back(name: &str, target: &Revision, newest: &Revision) -> String {
    let stays = format!("{name} stays on {}", display_version(&target.version));
    match &newest.liquidbounce {
        Some(range) => format!(
            "{stays}: {} needs {}.",
            display_version(&newest.version),
            range_label(range)
        ),
        None => format!("{stays}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::marketplace::ItemType;
    use crate::minecraft::progress::ProgressUpdate;
    use serde_json::json;

    #[test]
    fn picks_the_newest_revision_that_fits() {
        struct Log(std::cell::RefCell<Vec<String>>);
        impl ProgressReceiver for Log {
            fn progress_update(&self, _: ProgressUpdate) {}
            fn log(&self, msg: &str) {
                self.0.borrow_mut().push(msg.to_owned());
            }
        }

        let revision = |id, range: Option<(&str, &str)>| Revision {
            id,
            version: format!("1.{id}.0"),
            created_at: None,
            liquidbounce: range.map(|(min, max)| LiquidBounceRange {
                min: min.to_owned(),
                max: max.to_owned(),
            }),
        };
        let item = SubscribedItem {
            name: "Extras".to_owned(),
            id: 781,
            item_type: ItemType::Addon,
        };
        let log = Log(Default::default());
        let pick = |fitting: &[Revision], newest, installed| {
            pick(&item, "0.40.1", fitting, newest, installed, &log)
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
    fn reads_the_builds_a_revision_fits() {
        let revisions: Vec<Revision> = serde_json::from_value(json!([
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
