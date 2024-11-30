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

use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use path_absolutize::Absolutize;
use tokio::fs;

use crate::utils::{download_file, tar_gz_extract, zip_extract, OperatingSystem, OS};

use super::JavaDistribution;

/// Find java binary in JRE folder
pub async fn find_java_binary(
    runtimes_folder: &Path,
    jre_distribution: &JavaDistribution,
    jre_version: &u32,
) -> Result<PathBuf> {
    let runtime_path =
        runtimes_folder.join(format!("{}_{}", jre_distribution.get_name(), jre_version));

    // Find JRE in runtime folder
    let mut files = fs::read_dir(&runtime_path).await?;

    if let Some(jre_folder) = files.next_entry().await? {
        let folder_path = jre_folder.path();

        let java_binary = match OS {
            OperatingSystem::WINDOWS => folder_path.join("bin").join("javaw.exe"),
            OperatingSystem::OSX => folder_path
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java"),
            _ => folder_path.join("bin").join("java"),
        };

        if java_binary.exists() {
            // Check if the binary has execution permissions on linux and macOS
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                let metadata = fs::metadata(&java_binary).await?;

                if !metadata.permissions().mode() & 0o111 != 0 {
                    // try to change permissions
                    let mut permissions = metadata.permissions();
                    permissions.set_mode(0o111);
                    fs::set_permissions(&java_binary, permissions).await?;
                }
            }

            return Ok(java_binary.absolutize()?.to_path_buf());
        }
    }

    Err(anyhow::anyhow!("Failed to find JRE"))
}

/// Download specific JRE to runtimes
pub async fn jre_download<F>(
    runtimes_folder: &Path,
    jre_distribution: &JavaDistribution,
    jre_version: &u32,
    on_progress: F,
) -> Result<PathBuf>
where
    F: Fn(u64, u64),
{
    let runtime_path =
        runtimes_folder.join(format!("{}_{}", jre_distribution.get_name(), jre_version));

    if runtime_path.exists() {
        fs::remove_dir_all(&runtime_path).await?;
    }
    fs::create_dir_all(&runtime_path).await?;

    let url = jre_distribution.get_url(jre_version)?;
    let retrieved_bytes = download_file(&url, on_progress).await?;
    let cursor = Cursor::new(&retrieved_bytes[..]);

    match OS {
        OperatingSystem::WINDOWS => zip_extract(cursor, runtime_path.as_path()).await?,
        OperatingSystem::LINUX | OperatingSystem::OSX => {
            tar_gz_extract(cursor, runtime_path.as_path()).await?
        }
        _ => bail!("Unsupported OS"),
    }

    // Find JRE afterwards
    find_java_binary(runtimes_folder, jre_distribution, jre_version).await
}
