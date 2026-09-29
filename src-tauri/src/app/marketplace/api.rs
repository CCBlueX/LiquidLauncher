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
use chrono::NaiveDateTime;
use serde::Deserialize;

use super::ItemType;
use crate::app::client_api::{Build, Client, PaginatedResponse, API_V3};
use crate::app::client_api_target::CLIENT_BRANCH;

#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub downloads: u32,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Revision {
    pub id: u32,
    pub version: String,
    pub created_at: Option<NaiveDateTime>,
    /// The LiquidBounce versions of the builds it fits; `None` when none does.
    pub liquidbounce: Option<LiquidBounceRange>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LiquidBounceRange {
    pub min: String,
    pub max: String,
}

#[derive(Deserialize)]
struct Linked {
    item: Item,
}

/// Items of one type. The API leaves add-ons out of untyped listings, so LiquidBounce builds
/// predating add-ons never receive a type they cannot deserialize.
pub async fn items(
    client: &Client,
    item_type: &str,
    query: Option<&str>,
    limit: u32,
) -> Result<Vec<Item>> {
    let limit = limit.to_string();
    let mut params = vec![
        ("limit", limit.as_str()),
        ("branch", CLIENT_BRANCH),
        ("type", item_type),
    ];
    params.extend(query.map(|query| ("q", query)));

    let page: PaginatedResponse<Item> = client.request(API_V3, "marketplace", &params).await?;
    Ok(page.items)
}

pub async fn item(client: &Client, item_id: u32) -> Result<Item> {
    client
        .request_from_endpoint(API_V3, &format!("marketplace/{item_id}"))
        .await
}

/// The newest revisions of an item, whatever they fit.
pub async fn revisions(client: &Client, item_id: u32, limit: u32) -> Result<Vec<Revision>> {
    let path = format!("marketplace/{item_id}/revisions");
    let page: PaginatedResponse<Revision> = client
        .request(API_V3, &path, &[("limit", &limit.to_string())])
        .await?;
    Ok(page.items)
}

pub async fn revision(client: &Client, item_id: u32, revision_id: u32) -> Result<Revision> {
    client
        .request_from_endpoint(
            API_V3,
            &format!("marketplace/{item_id}/revisions/{revision_id}"),
        )
        .await
}

/// Every revision that fits the build, newest first.
pub async fn revisions_for(client: &Client, item_id: u32, build: &Build) -> Result<Vec<Revision>> {
    let path = format!("marketplace/{item_id}/revisions");
    let mut revisions = vec![];
    for page in 1.. {
        let response: PaginatedResponse<Revision> = client
            .request(
                API_V3,
                &path,
                &[
                    ("minecraft", &build.mc_version),
                    ("liquidbounce", &build.lb_version),
                    ("page", &page.to_string()),
                    ("limit", "50"),
                ],
            )
            .await?;
        revisions.extend(response.items);
        if page >= response.pagination.pages {
            break;
        }
    }
    Ok(revisions)
}

/// The items an item needs installed beside it.
pub async fn dependencies(client: &Client, item_id: u32) -> Result<Vec<Item>> {
    let linked: Vec<Linked> = client
        .request_from_endpoint(API_V3, &format!("marketplace/{item_id}/dependencies"))
        .await?;
    Ok(linked.into_iter().map(|linked| linked.item).collect())
}

pub fn download_url(client: &Client, item_id: u32, revision_id: u32) -> String {
    format!(
        "{}/{API_V3}/marketplace/{item_id}/revisions/{revision_id}/download",
        client.url()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_an_item_needs() {
        // The shape of GET /api/v3/marketplace/{id}/dependencies.
        let linked: Vec<Linked> = serde_json::from_value(serde_json::json!([{
            "item": {
                "id": 771,
                "uid": "d560440229a5a1d5",
                "type": "Addon",
                "name": "ScriptAPI",
                "branch": "nextgen",
                "description": "The JavaScript Script API for LiquidBounce.",
                "thumbnail_pid": null,
                "featured": false,
                "created_at": "2026-09-17T10:00:00",
                "status": "Active"
            },
            "author": "1zun4",
            "live_revision": { "id": 4323, "item_id": 771, "version": "v1.0.0" }
        }]))
        .unwrap();
        assert_eq!(linked[0].item.id, 771);
        assert_eq!(linked[0].item.item_type, ItemType::Addon);
    }
}
