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

use std::path::PathBuf;

use anyhow::{Context, Result};
use tauri::State;

use crate::app::client_api::{Build, Client};
use crate::app::client_api_target::CLIENT_BRANCH;
use crate::app::gui::AppState;
use crate::app::marketplace::view::{self, Browse, Detail, Library, Remote};
use crate::app::marketplace::{self, MarketplaceItemType, SubscribedItem};
use crate::app::options::Options;
use crate::LAUNCHER_DIRECTORY;

/// Takes the options from the frontend, like `run_client`: Settings stores them only once it
/// closes, so the stored ones may point at another data directory.
pub(crate) fn data_directory(options: &Options) -> PathBuf {
    match options.start_options.custom_data_path.as_str() {
        "" => LAUNCHER_DIRECTORY.data_dir().to_path_buf(),
        path => PathBuf::from(path),
    }
}

/// The build selected to launch, resolved like the main screen does.
fn cached_build(options: &Options, state: &AppState) -> Option<Build> {
    let builds = state.builds.lock().ok()?;
    select(&builds, options.version_options.build_id).cloned()
}

fn select(builds: &[Build], build_id: i32) -> Option<&Build> {
    match build_id {
        -1 => builds.first(),
        id => builds
            .iter()
            .find(|build| i64::from(build.build_id) == i64::from(id)),
    }
}

async fn selected_build(client: &Client, options: &Options, state: &AppState) -> Result<Build> {
    if let Some(build) = cached_build(options, state) {
        return Ok(build);
    }

    let builds = client
        .builds(!options.launcher_options.show_nightly_builds)
        .await?;
    select(&builds, options.version_options.build_id)
        .cloned()
        .context("The selected build is not available")
}

/// The installed themes and add-ons. Without `check` it reads the disk alone and returns at once.
#[tauri::command]
pub(crate) async fn get_marketplace_library(
    client: Client,
    options: Options,
    check: bool,
    app_state: State<'_, AppState>,
) -> Result<Library, String> {
    let data = data_directory(&options);
    let library = if check {
        match selected_build(&client, &options, &app_state).await {
            Ok(build) => {
                view::library(&data, CLIENT_BRANCH, Some(&build), Remote::Check(&client)).await
            }
            Err(error) => {
                let error = format!(
                    "unable to check the marketplace: {}",
                    view::describe(&error)
                );
                view::library(&data, CLIENT_BRANCH, None, Remote::Failed(error)).await
            }
        }
    } else {
        let build = cached_build(&options, &app_state);
        view::library(&data, CLIENT_BRANCH, build.as_ref(), Remote::Skip).await
    };

    library.map_err(|e| {
        format!(
            "unable to read marketplace subscriptions: {}",
            view::describe(&e)
        )
    })
}

#[tauri::command]
pub(crate) async fn browse_marketplace(
    client: Client,
    options: Options,
    item_type: MarketplaceItemType,
    query: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Browse, String> {
    async {
        let build = selected_build(&client, &options, &app_state).await?;
        let query = query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty());
        view::browse(
            &client,
            &data_directory(&options),
            CLIENT_BRANCH,
            &build,
            item_type,
            query,
        )
        .await
    }
    .await
    .map_err(|e| format!("unable to browse marketplace: {}", view::describe(&e)))
}

#[tauri::command]
pub(crate) async fn get_marketplace_item(
    client: Client,
    options: Options,
    item_id: u32,
    app_state: State<'_, AppState>,
) -> Result<Detail, String> {
    async {
        let build = selected_build(&client, &options, &app_state).await?;
        view::detail(
            &client,
            &data_directory(&options),
            CLIENT_BRANCH,
            &build,
            item_id,
        )
        .await
    }
    .await
    .map_err(|e| format!("unable to load marketplace item: {}", view::describe(&e)))
}

/// Installs a theme at once, and an add-on in the revision that fits the selected build. While the
/// game runs, both wait until it exits.
#[tauri::command]
pub(crate) async fn install_marketplace_item(
    client: Client,
    options: Options,
    item_id: u32,
    name: String,
    item_type: MarketplaceItemType,
    app_state: State<'_, AppState>,
) -> Result<(), String> {
    let item = SubscribedItem {
        name,
        id: item_id,
        item_type,
    };

    async {
        let revision = if marketplace::game_running() {
            None
        } else if item_type == MarketplaceItemType::Addon {
            let build = selected_build(&client, &options, &app_state).await?;
            Some(view::install_target(&client, &build, &item).await?)
        } else {
            let newest = client.marketplace_revisions(item_id, 1).await?;
            let newest = newest.items.first().context("Nothing is published yet")?;
            Some(newest.id)
        };

        marketplace::add(
            &client,
            &data_directory(&options),
            CLIENT_BRANCH,
            item,
            revision,
        )
        .await
    }
    .await
    .map_err(|e| format!("unable to install marketplace item: {}", view::describe(&e)))
}

#[tauri::command]
pub(crate) async fn remove_marketplace_item(options: Options, item_id: u32) -> Result<(), String> {
    marketplace::remove(&data_directory(&options), CLIENT_BRANCH, item_id)
        .await
        .map_err(|e| format!("unable to remove marketplace item: {}", view::describe(&e)))
}
