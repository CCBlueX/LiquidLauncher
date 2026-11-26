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

//! English and Simplified Chinese ship with the launcher. The other languages come from
//! CCBlueX/Translations, published per launcher version so a release never shows wording written
//! for a later one.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::warn;

use crate::{HTTP_CLIENT, LAUNCHER_DIRECTORY, LAUNCHER_VERSION};

const HOST: &str = "https://i18n.liquidbounce.net/launcher";

#[derive(Serialize, Deserialize, Clone)]
pub struct Language {
    /// BCP 47, `de` or `zh-CN`.
    code: String,
    /// In the language itself.
    name: String,
    /// Minecraft's code, `de_de`.
    minecraft: String,
    file: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Index {
    languages: Vec<Language>,
}

/// The translations this build reads: its version's, or `next` before a release.
fn channel() -> &'static str {
    match option_env!("TRANSLATIONS_CHANNEL") {
        Some(channel) if !channel.is_empty() => channel,
        _ if cfg!(debug_assertions) => "next",
        _ => LAUNCHER_VERSION,
    }
}

fn cache() -> PathBuf {
    LAUNCHER_DIRECTORY
        .data_dir()
        .join("translations")
        .join(channel())
}

async fn fetch(file: &str) -> Result<Vec<u8>> {
    let response = HTTP_CLIENT
        .get(format!("{HOST}/{}/{file}", channel()))
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

/// The languages this version has besides English, as last seen when the host is unreachable.
pub async fn languages() -> Vec<Language> {
    match fetch_index().await {
        Ok(index) => index.languages,
        Err(error) => {
            warn!("Failed to fetch the translation index: {error:#}");
            cached_index()
                .await
                .map(|index| index.languages)
                .unwrap_or_default()
        }
    }
}

async fn fetch_index() -> Result<Index> {
    let json = fetch("index.json").await?;
    let index = serde_json::from_slice(&json)?;
    fs::create_dir_all(cache()).await?;
    fs::write(cache().join("index.json"), json).await?;
    Ok(index)
}

async fn cached_index() -> Option<Index> {
    let json = fs::read(cache().join("index.json")).await.ok()?;
    serde_json::from_slice(&json).ok()
}

/// The messages of a language from the last index, downloaded when they changed.
pub async fn translation(code: &str) -> Result<Value> {
    let language = cached_index()
        .await
        .and_then(|index| index.languages.into_iter().find(|language| language.code == code))
        .with_context(|| format!("No translation for {code}"))?;
    // The host names the file, so it must not reach outside the cache.
    if Path::new(&language.file).file_name() != Some(language.file.as_ref()) {
        bail!("Invalid translation file name {}", language.file);
    }

    let path = cache().join(&language.file);
    let cached = fs::read(&path).await.ok();
    let json = match cached {
        Some(json) if sha256(&json) == language.sha256 => json,
        cached => match download(&language).await {
            Ok(json) => {
                fs::write(&path, &json).await?;
                json
            }
            Err(error) => {
                warn!("Failed to download the {code} translation: {error:#}");
                cached.context("No cached translation")?
            }
        },
    };
    Ok(serde_json::from_slice(&json)?)
}

async fn download(language: &Language) -> Result<Vec<u8>> {
    let json = fetch(&language.file).await?;
    if sha256(&json) != language.sha256 {
        bail!("Checksum mismatch");
    }
    Ok(json)
}

fn sha256(bytes: &[u8]) -> String {
    base16ct::lower::encode_string(&Sha256::digest(bytes))
}
