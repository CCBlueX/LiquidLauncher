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

//! The marketplace endpoints of the LiquidBounce API.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::ItemType;
use crate::app::client_api::{Build, Client, PaginatedResponse};
use crate::app::client_api_target::CLIENT_EDITION;

#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub description: String,
    pub owner: Owner,
    #[serde(default)]
    pub downloads: u32,
    #[serde(rename = "liveRevision")]
    pub live_revision: Option<Revision>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Owner {
    pub nickname: Option<String>,
    pub uid: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Revision {
    pub id: u32,
    pub version: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    /// The LiquidBounce versions of the builds it fits; `None` when none does.
    pub liquidbounce: Option<LiquidBounceRange>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LiquidBounceRange {
    pub min: String,
    pub max: String,
}

#[derive(Deserialize)]
struct Dependencies {
    requires: Vec<Item>,
}

pub async fn items(
    client: &Client,
    item_type: &str,
    query: Option<&str>,
    limit: u32,
) -> Result<Vec<Item>> {
    let limit = limit.to_string();
    let mut params = vec![
        ("limit", limit.as_str()),
        ("edition", CLIENT_EDITION),
        ("type", item_type),
    ];
    params.extend(query.map(|query| ("q", query)));

    let page: PaginatedResponse<Item> = client.request("marketplace/items", &params).await?;
    Ok(page.items)
}

pub async fn item(client: &Client, item_id: u32) -> Result<Item> {
    client
        .request(&format!("marketplace/items/{item_id}"), &[])
        .await
}

/// The newest revisions of an item, whatever they fit.
pub async fn revisions(client: &Client, item_id: u32, limit: u32) -> Result<Vec<Revision>> {
    let path = format!("marketplace/items/{item_id}/revisions");
    let page: PaginatedResponse<Revision> = client
        .request(&path, &[("limit", &limit.to_string())])
        .await?;
    Ok(page.items)
}

pub async fn revision(client: &Client, item_id: u32, revision_id: u32) -> Result<Revision> {
    client
        .request(
            &format!("marketplace/items/{item_id}/revisions/{revision_id}"),
            &[],
        )
        .await
}

/// Every revision that fits the build, newest first.
pub async fn revisions_for(client: &Client, item_id: u32, build: &Build) -> Result<Vec<Revision>> {
    let path = format!("marketplace/items/{item_id}/revisions");
    client.request_pages(&path, &[("minecraft", &build.mc_version), ("liquidbounce", &build.lb_version)]).await
}

/// The items an item needs installed beside it.
pub async fn dependencies(client: &Client, item_id: u32) -> Result<Vec<Item>> {
    let linked: Dependencies = client
        .request(&format!("marketplace/items/{item_id}"), &[])
        .await?;
    Ok(linked.requires)
}

pub fn download_url(client: &Client, item_id: u32, revision_id: u32) -> String {
    format!(
        "{}/marketplace/items/{item_id}/revisions/{revision_id}/file",
        client.url()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_an_item_needs() {
        let linked: Dependencies = serde_json::from_value(serde_json::json!({
            "dependencies": [{"id": 771}],
            "requires": [{
                "id": 771, "type": "addon", "name": "ScriptAPI", "description": "",
                "owner": { "uid": "author" }
            }]
        }))
        .unwrap();
        assert_eq!(linked.requires[0].id, 771);
        assert_eq!(linked.requires[0].item_type, ItemType::Addon);
    }
}
