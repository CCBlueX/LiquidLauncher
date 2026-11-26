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
use tokio::fs;

use crate::app::client_api_target::CLIENT_EDITION;

/// The game directory of the edition: `<data>/instances/<edition>`. Every build of the edition runs
/// in it, whatever branch it was built from.
pub struct GameDir {
    data: PathBuf,
}

impl GameDir {
    pub fn new(data: impl Into<PathBuf>) -> Self {
        Self { data: data.into() }
    }

    /// Where the game directories of all editions are.
    pub fn root(data: &Path) -> PathBuf {
        data.join("instances")
    }

    /// Renames the `gameDir` an older launcher left in `data` to what [`GameDir::root`] is now.
    /// An install that has both keeps both: nothing is merged.
    pub async fn migrate(data: &Path) -> Result<()> {
        let old = data.join("gameDir");
        let new = Self::root(data);

        let moved = async {
            if fs::try_exists(&new).await? || !fs::try_exists(&old).await? {
                return Ok(());
            }
            fs::rename(&old, &new).await
        };
        moved
            .await
            .with_context(|| format!("Failed to move {} to {}", old.display(), new.display()))
    }

    pub fn path(&self) -> PathBuf {
        Self::root(&self.data).join(CLIENT_EDITION)
    }

    pub fn mods(&self) -> PathBuf {
        self.path().join("mods")
    }

    /// The LiquidBounce folder: configs, the client account, marketplace items.
    pub fn client(&self) -> PathBuf {
        self.path().join("LiquidBounce")
    }

    pub fn subscriptions(&self) -> PathBuf {
        self.client().join("marketplace.json")
    }

    pub fn item(&self, item_id: u32) -> PathBuf {
        self.client()
            .join("marketplace")
            .join("items")
            .join(item_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn moves_the_game_directories_of_an_older_launcher() -> Result<()> {
        let data = tempfile::tempdir()?;
        fs::create_dir_all(data.path().join("gameDir/nextgen/LiquidBounce")).await?;

        GameDir::migrate(data.path()).await?;

        assert!(!fs::try_exists(data.path().join("gameDir")).await?);
        assert!(fs::try_exists(GameDir::new(data.path()).client()).await?);
        Ok(())
    }

    #[tokio::test]
    async fn leaves_both_alone_when_the_new_directory_exists() -> Result<()> {
        let data = tempfile::tempdir()?;
        fs::create_dir_all(data.path().join("gameDir/legacy")).await?;
        fs::create_dir_all(data.path().join("instances/nextgen")).await?;

        GameDir::migrate(data.path()).await?;

        assert!(fs::try_exists(data.path().join("gameDir/legacy")).await?);
        assert!(!fs::try_exists(data.path().join("instances/legacy")).await?);
        Ok(())
    }

    #[tokio::test]
    async fn does_nothing_on_a_fresh_install() -> Result<()> {
        let data = tempfile::tempdir()?;

        GameDir::migrate(data.path()).await?;

        assert!(!fs::try_exists(GameDir::root(data.path())).await?);
        Ok(())
    }
}
