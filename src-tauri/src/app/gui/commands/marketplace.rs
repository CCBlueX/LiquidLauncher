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

use crate::app::client_api::{Client, MarketplaceItem, PaginatedResponse};
use crate::app::client_api_target::CLIENT_BRANCH;
use crate::app::marketplace::{self, MarketplaceItemType, SubscribedItem};
use crate::app::options::Options;
use crate::LAUNCHER_DIRECTORY;

/// Takes the options from the frontend, like `run_client`: Settings stores them only once it
/// closes, so the stored ones may point at another data directory.
fn data_directory(options: &Options) -> PathBuf {
    match options.start_options.custom_data_path.as_str() {
        "" => LAUNCHER_DIRECTORY.data_dir().to_path_buf(),
        path => PathBuf::from(path),
    }
}

/// What LiquidBounce currently has subscribed, add-ons and themes alike.
#[tauri::command]
pub(crate) async fn get_marketplace_subscriptions(
    options: Options,
) -> Result<Vec<SubscribedItem>, String> {
    marketplace::read_subscriptions(&data_directory(&options), CLIENT_BRANCH)
        .await
        .map_err(|e| format!("unable to read marketplace subscriptions: {:?}", e))
}

#[tauri::command]
pub(crate) async fn browse_marketplace_items(
    client: Client,
    page: u32,
    limit: u32,
    query: Option<String>,
    item_type: Option<String>,
) -> Result<PaginatedResponse<MarketplaceItem>, String> {
    client
        .marketplace_items(page, limit, query.as_deref(), item_type.as_deref())
        .await
        .map_err(|e| format!("unable to browse marketplace: {:?}", e))
}

/// Subscribes to an item and installs its newest revision.
#[tauri::command]
pub(crate) async fn subscribe_marketplace_item(
    client: Client,
    options: Options,
    item_id: u32,
    name: String,
    item_type: MarketplaceItemType,
) -> Result<(), String> {
    let revisions = client
        .marketplace_revisions(item_id)
        .await
        .map_err(|e| format!("unable to resolve revisions: {:?}", e))?;

    let revision = revisions
        .items
        .first()
        .ok_or_else(|| "item has no published revision".to_string())?;

    let data = data_directory(&options);
    marketplace::install(
        &data,
        CLIENT_BRANCH,
        item_id,
        revision.id,
        &client.marketplace_download_url(item_id, revision.id),
    )
    .await
    .map_err(|e| format!("unable to install marketplace item: {:?}", e))?;

    let item = SubscribedItem {
        name,
        id: item_id,
        item_type,
    };
    marketplace::subscribe(&data, CLIENT_BRANCH, &item)
        .await
        .map_err(|e| format!("unable to subscribe to marketplace item: {:?}", e))
}

#[tauri::command]
pub(crate) async fn unsubscribe_marketplace_item(
    options: Options,
    item_id: u32,
) -> Result<(), String> {
    marketplace::uninstall(&data_directory(&options), CLIENT_BRANCH, item_id)
        .await
        .map_err(|e| format!("unable to remove marketplace item: {:?}", e))
}
