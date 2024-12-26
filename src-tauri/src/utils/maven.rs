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

use crate::error::LauncherError;
use anyhow::Result;

pub fn get_maven_artifact_path(artifact_id: &String) -> Result<String> {
    let split = artifact_id.split(':').collect::<Vec<_>>();

    if split.len() != 3 {
        return Err(LauncherError::InvalidVersionProfile(format!(
            "Invalid artifact name: {}",
            artifact_id
        ))
        .into());
    }

    Ok(format!(
        "{}/{name}/{ver}/{name}-{ver}.jar",
        split[0].replace('.', "/"),
        name = split[1],
        ver = split[2]
    ))
}
