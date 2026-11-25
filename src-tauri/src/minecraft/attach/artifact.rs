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

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use modloader::VERSION;
use semver::Version;

use crate::app::client_api::LaunchManifest;
use crate::minecraft::prelauncher::{repository_jar, repository_url};
use crate::minecraft::progress::ProgressReceiver;
use crate::utils::{download_file, maven_versions};

pub async fn artifact(
    data: &Path,
    manifest: &LaunchManifest,
    name: &str,
    progress: &impl ProgressReceiver,
) -> Result<PathBuf> {
    let repository = "ccbluex";
    let url = format!(
        "{}net/ccbluex/{name}/maven-metadata.xml",
        repository_url(manifest, repository)?
    );
    let metadata = String::from_utf8(download_file(&url, |_, _| {}).await?)?;
    let loader = Version::parse(VERSION)?;
    let version = newest(&maven_versions(&metadata)?, &loader)
        .with_context(|| format!("No {name} {}.{} is released", loader.major, loader.minor))?;

    let artifact = format!("net.ccbluex:{name}:{version}");
    repository_jar(data, manifest, repository, &artifact, progress, |_, _| {}).await
}

fn newest(versions: &[String], loader: &Version) -> Option<Version> {
    versions
        .iter()
        .filter_map(|version| Version::parse(version).ok())
        .filter(|version| version.pre.is_empty())
        .filter(|version| (version.major, version.minor) == (loader.major, loader.minor))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_newest_release_of_the_same_major_and_minor() {
        let versions: Vec<String> = [
            "0.1.9",
            "0.2.0",
            "0.2.10",
            "0.2.9",
            "0.2.11-SNAPSHOT",
            "0.3.0",
            "1.2.0",
        ]
        .map(String::from)
        .into();
        let loader = Version::new(0, 2, 0);

        assert_eq!(newest(&versions, &loader), Some(Version::new(0, 2, 10)));
        assert_eq!(newest(&["0.1.0".into(), "0.3.0".into()], &loader), None);
    }
}
