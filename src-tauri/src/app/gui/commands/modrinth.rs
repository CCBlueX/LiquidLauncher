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

use tauri::State;
use tracing::warn;

use super::client::custom_mods_dir;
use super::marketplace::selected_build;
use crate::app::client_api::Client;
use crate::app::gui::AppState;
use crate::app::marketplace::view::describe;
use crate::app::modrinth::{self, Held, ModrinthMod, SearchHit};
use crate::app::options::Options;

/// Modrinth mods for the selected build, each with what installing it would do.
#[tauri::command]
pub(crate) async fn modrinth_search(
    client: Client,
    options: Options,
    query: Option<String>,
    app_state: State<'_, AppState>,
) -> Result<Vec<SearchHit>, String> {
    async {
        let build = selected_build(&client, &options, &app_state).await?;
        let query = query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty());
        // A file the user added counts as installed once Modrinth knows it.
        let files = async {
            let dir = custom_mods_dir(&build.branch, &build.mc_version);
            anyhow::Ok(
                modrinth::file_projects(&dir)
                    .await
                    .inspect_err(|error| warn!("Failed to look up mod files: {:?}", error))
                    .unwrap_or_default(),
            )
        };
        let (hits, manifest, mut recommended, mut installed) = tokio::try_join!(
            modrinth::search(query, &build.mc_version, &build.subsystem),
            client.fetch_launch_manifest(build.build_id),
            client.fetch_mods(&build.mc_version, &build.subsystem),
            files,
        )?;

        let branch = options
            .version_options
            .options
            .get(&build.branch)
            .cloned()
            .unwrap_or_default();
        for recommended in &mut recommended {
            if let Some(enabled) = branch.mod_states.get(&recommended.name) {
                recommended.enabled = *enabled;
            }
        }
        installed.extend(
            branch
                .modrinth_mods
                .get(&build.mc_version)
                .into_iter()
                .flatten()
                .map(|installed| installed.project_id.clone()),
        );

        anyhow::Ok(modrinth::tell(
            hits,
            &Held {
                launched: &manifest.mods,
                recommended: &recommended,
                installed: &installed,
            },
        ))
    }
    .await
    .map_err(|e| format!("unable to search Modrinth: {}", describe(&e)))
}

/// The newest version of a project for the selected build, to install or to update to.
#[tauri::command]
pub(crate) async fn modrinth_install(
    client: Client,
    options: Options,
    project_id: String,
    app_state: State<'_, AppState>,
) -> Result<ModrinthMod, String> {
    async {
        let build = selected_build(&client, &options, &app_state).await?;
        modrinth::resolve(&project_id, &build.mc_version, &build.subsystem).await
    }
    .await
    .map_err(|e| format!("unable to install from Modrinth: {}", describe(&e)))
}
