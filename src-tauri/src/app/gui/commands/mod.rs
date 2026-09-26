/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2024 CCBlueX
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

pub(crate) mod auth;
pub(crate) mod client;
pub(crate) mod data;
pub(crate) mod marketplace;
pub(crate) mod system;
pub(crate) mod minecraft_installation;
pub(crate) mod updater;
pub(crate) mod modrinth;

pub(crate) use auth::*;
pub(crate) use client::*;
pub(crate) use data::*;
pub(crate) use marketplace::*;
pub(crate) use system::*;
pub(crate) use minecraft_installation::*;
pub(crate) use updater::*;
pub(crate) use modrinth::*;

use crate::utils::error_line;

/// Turns an error into the message the frontend shows: `unable to <action>: <error>`.
fn failed(action: &str) -> impl FnOnce(anyhow::Error) -> String + '_ {
    move |error| format!("unable to {action}: {}", error_line(&error))
}

/// A search query, or `None` when it is blank.
fn search_query(query: &Option<String>) -> Option<&str> {
    query.as_deref().map(str::trim).filter(|query| !query.is_empty())
}
