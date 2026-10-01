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

use anyhow::{bail, Context, Result};
use path_absolutize::Absolutize;
use tokio::fs;
use tracing::info;

use crate::utils::{
    download_file, tar_gz_extract, zip_extract, Architecture, OperatingSystem, ARCHITECTURE, OS,
};

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

/// Like [find_java_binary], but a runtime for another architecture counts as missing. Launchers
/// that ran emulated (x64 under Rosetta or on Windows ARM) cached x64 runtimes under the same name.
pub async fn find_cached_java_binary(
    runtimes_folder: &Path,
    jre_distribution: &JavaDistribution,
    jre_version: &u32,
) -> Result<PathBuf> {
    let java_binary = find_java_binary(runtimes_folder, jre_distribution, jre_version).await?;

    if let Some(architecture) = runtime_architecture(&java_binary).await {
        if architecture != *ARCHITECTURE {
            info!(
                "Cached {} {} runtime is for {}, not {}",
                jre_distribution.get_name(),
                jre_version,
                architecture,
                *ARCHITECTURE
            );
            bail!("The cached runtime is for {}", architecture);
        }
    }

    Ok(java_binary)
}

/// From the `release` file next to `bin`, if it names a known architecture.
async fn runtime_architecture(java_binary: &Path) -> Option<Architecture> {
    let home = java_binary.parent()?.parent()?;
    let release = fs::read_to_string(home.join("release")).await.ok()?;
    let arch = release
        .lines()
        .find_map(|line| line.strip_prefix("OS_ARCH="))?
        .trim_matches('"');

    Some(match arch {
        "x86_64" | "amd64" => Architecture::X64,
        "aarch64" | "arm64" => Architecture::AARCH64,
        "x86" | "i386" | "i586" | "i686" => Architecture::X86,
        "arm" => Architecture::ARM,
        _ => return None,
    })
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

    let url = jre_distribution
        .get_url(jre_version)
        .await?
        .with_context(|| {
            format!(
                "{} has no Java {} build for {}-{}",
                jre_distribution.get_name(),
                jre_version,
                OS,
                *ARCHITECTURE
            )
        })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn architecture_of(release: Option<&str>) -> Option<Architecture> {
        let home = std::env::temp_dir().join(format!(
            "liquidlauncher-test-{}-release",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("bin")).unwrap();
        if let Some(release) = release {
            std::fs::write(home.join("release"), release).unwrap();
        }

        let architecture = runtime_architecture(&home.join("bin").join("java")).await;
        std::fs::remove_dir_all(&home).unwrap();
        architecture
    }

    #[tokio::test]
    async fn reads_the_architecture_of_a_runtime() {
        let release = |arch: &str| {
            format!("JAVA_VERSION=\"25.0.4.1\"\nOS_ARCH=\"{arch}\"\nOS_NAME=\"Linux\"\n")
        };

        assert_eq!(
            architecture_of(Some(&release("x86_64"))).await,
            Some(Architecture::X64)
        );
        assert_eq!(
            architecture_of(Some(&release("aarch64"))).await,
            Some(Architecture::AARCH64)
        );
        // Java 8
        assert_eq!(
            architecture_of(Some("OS_ARCH=\"amd64\"")).await,
            Some(Architecture::X64)
        );
        assert_eq!(architecture_of(Some("JAVA_VERSION=\"25\"")).await, None);
        assert_eq!(architecture_of(None).await, None);
    }
}
