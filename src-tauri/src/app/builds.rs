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

//! The build that launches, and the builds to choose from as the Client tab lists them.

use std::sync::Mutex;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::app::client_api::{Build, Client};
use crate::app::options::Options;

const PAGE_SIZE: u32 = 20;

/// The build that launches: the newest for "Latest" (`-1`), a release unless `nightly`, or the
/// chosen one.
async fn fetch(client: &Client, build_id: i32, nightly: bool) -> Result<Build> {
    match u32::try_from(build_id) {
        Ok(build_id) => client.build(build_id).await,
        Err(_) => client
            .build_page(1, 1, nightly)
            .await?
            .items
            .into_iter()
            .next()
            .context("No build is available"),
    }
}

/// The build that launches, as fetched last for a choice of build and nightly builds.
pub struct SelectedBuild {
    build_id: i32,
    nightly: bool,
    build: Build,
}

impl SelectedBuild {
    /// A chosen build does not change with the nightly builds shown, "Latest" does.
    fn is_for(&self, build_id: i32, nightly: bool) -> bool {
        self.build_id == build_id && (build_id != -1 || self.nightly == nightly)
    }
}

pub type KeptBuild = Mutex<Option<SelectedBuild>>;

/// The build that launches as kept from last time, if it is for the same choice.
pub(crate) fn kept(options: &Options, kept: &KeptBuild) -> Option<Build> {
    let kept = kept.lock().ok()?;
    kept.as_ref()
        .filter(|kept| {
            kept.is_for(
                options.version_options.build_id,
                options.launcher_options.show_nightly_builds,
            )
        })
        .map(|kept| kept.build.clone())
}

/// Fetches the build that launches and keeps it for the views that follow.
pub(crate) async fn fetch_selected(
    client: &Client,
    options: &Options,
    kept: &KeptBuild,
) -> Result<Build> {
    let build_id = options.version_options.build_id;
    let nightly = options.launcher_options.show_nightly_builds;
    let build = fetch(client, build_id, nightly).await?;
    if let Ok(mut kept) = kept.lock() {
        *kept = Some(SelectedBuild {
            build_id,
            nightly,
            build: build.clone(),
        });
    }
    Ok(build)
}

/// The build that launches, as kept or fetched.
pub(crate) async fn selected(
    client: &Client,
    options: &Options,
    kept: &KeptBuild,
) -> Result<Build> {
    match self::kept(options, kept) {
        Some(build) => Ok(build),
        None => fetch_selected(client, options, kept).await,
    }
}

/// How the build that launches was chosen. The frontend shows `Latest release`, or when it was
/// pinned `Sep 26 · adfddc0`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    latest: bool,
    release: bool,
    date: DateTime<Utc>,
    commit: String,
}

pub fn selection(build: &Build, latest: bool) -> Selection {
    Selection {
        latest,
        release: build.release,
        date: build.date,
        commit: commit(build),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildChoice {
    build_id: u32,
    /// `0.40.1 · Minecraft 26.3`
    name: String,
    /// The first line of the commit message.
    description: String,
    date: DateTime<Utc>,
    commit: String,
    selected: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPage {
    /// Whether "Latest" is selected rather than a build.
    latest: bool,
    builds: Vec<BuildChoice>,
    page: u32,
    pages: u32,
    /// Builds across all pages, releases unless `nightly`.
    total: u32,
    nightly: bool,
}

/// One page of the builds, newest first. `selected` is the chosen build id, `-1` for the latest.
pub async fn page(client: &Client, page: u32, nightly: bool, selected: i32) -> Result<BuildPage> {
    let response = client.build_page(page, PAGE_SIZE, nightly).await?;
    let builds = response
        .items
        .iter()
        .map(|build| BuildChoice {
            build_id: build.build_id,
            name: format!("{} · Minecraft {}", build.lb_version, build.mc_version),
            description: build.message.lines().next().unwrap_or_default().to_owned(),
            date: build.date,
            commit: commit(build),
            selected: i64::from(build.build_id) == i64::from(selected),
        })
        .collect();

    Ok(BuildPage {
        latest: selected == -1,
        builds,
        page,
        pages: response.total.div_ceil(PAGE_SIZE),
        total: response.total,
        nightly,
    })
}

/// The commit a build was made from, `adfddc0`.
fn commit(build: &Build) -> String {
    build.commit_id.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_build_of_the_paginated_endpoint() {
        let build: Build = serde_json::from_value(serde_json::json!({
            "id": 17371,
            "commitId": "adfddc0781d056201037ac16b216cfd3e2df4e4a",
            "edition": "nextgen",
            "subsystem": "fabric",
            "lbVersion": "0.40.1",
            "mcVersion": "26.2",
            "release": false,
            "committedAt": "2026-09-23T13:24:01Z",
            "message": "refactor(interop): switch engine to CIO (#9233)\n\nBecause we don't need HTTP2",
            "jreVersion": 25,
            "jreDistribution": "temurin",
            "fabricApiVersion": "0.153.0+26.2",
            "fabricLoaderVersion": "0.19.3",
            "kotlinVersion": "2.4.0",
            "kotlinModVersion": "1.13.12+kotlin.2.4.0",
            "downloadUrl": "https://liquidbounce.net/download/queue/17371"
        }))
        .unwrap();

        assert_eq!(commit(&build), "adfddc0");
        let latest = selection(&build, true);
        assert!(latest.latest && !latest.release);
        assert_eq!(latest.commit, "adfddc0");
        assert!(!selection(&build, false).latest);

        let latest = SelectedBuild {
            build_id: -1,
            nightly: true,
            build: build.clone(),
        };
        assert!(latest.is_for(-1, true));
        assert!(!latest.is_for(-1, false));
        assert!(!latest.is_for(17371, true));

        let pinned = SelectedBuild {
            build_id: 17371,
            nightly: true,
            build,
        };
        assert!(pinned.is_for(17371, false));
        assert!(!pinned.is_for(-1, true));
    }
}
