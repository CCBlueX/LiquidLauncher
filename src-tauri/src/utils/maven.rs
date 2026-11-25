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
use quick_xml::de::from_str;
use serde::Deserialize;

pub fn get_maven_artifact_path(artifact_id: &str) -> Result<String> {
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

#[derive(Deserialize)]
struct Metadata {
    versioning: Versioning,
}

#[derive(Deserialize)]
struct Versioning {
    versions: Versions,
}

#[derive(Deserialize)]
struct Versions {
    #[serde(default)]
    version: Vec<String>,
}

pub fn maven_versions(metadata: &str) -> Result<Vec<String>> {
    let metadata: Metadata = from_str(metadata)?;
    Ok(metadata.versioning.versions.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_versions_of_an_artifact() -> Result<()> {
        let versions = maven_versions(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <metadata>
              <groupId>net.ccbluex</groupId>
              <artifactId>modloader</artifactId>
              <versioning>
                <latest>0.2.0</latest>
                <release>0.2.0</release>
                <versions>
                  <version>0.1.0</version>
                  <version>0.2.0</version>
                </versions>
                <lastUpdated>20261125101112</lastUpdated>
              </versioning>
            </metadata>"#,
        )?;

        assert_eq!(versions, ["0.1.0", "0.2.0"]);
        Ok(())
    }
}
