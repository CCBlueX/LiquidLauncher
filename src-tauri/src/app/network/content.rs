/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2025 CCBlueX
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

use crate::HTTP_CLIENT;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const CONTENT_DELIVERY: &str = "https://cloud.liquidbounce.net";
pub const CONTENT_FOLDER: &str = "LiquidLauncher";

pub struct ContentDelivery;

impl ContentDelivery {

    /// Fetch news from Cloud Content Delivery and deserialize to list of [NewsArticle].
    pub async fn news() -> anyhow::Result<Vec<NewsArticle>> {
        Self::request_from_content_delivery("news.json").await
    }

    /// Request JSON data from content delivery and deserialize to type [T].
    pub async fn request_from_content_delivery<T: DeserializeOwned>(file: &str) -> anyhow::Result<T> {
        Ok(HTTP_CLIENT
            .get(format!("{}/{}/{}", CONTENT_DELIVERY, CONTENT_FOLDER, file))
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await?)
    }

}

/// News Article
#[derive(Serialize, Deserialize)]
pub struct NewsArticle {
    pub title: String,
    pub description: String,
    pub date: String,
    pub url: String,
    #[serde(rename = "bannerText")]
    pub banner_text: String,
    #[serde(rename = "bannerUrl")]
    pub banner_url: String,
}