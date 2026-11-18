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

use std::fs::File;
use std::io::{Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use chrono::Utc;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
use tokio::{fs, process::Command};
use tracing::warn;

use crate::utils::{
    download_file, tar_gz_extract, zip_extract, Architecture, OperatingSystem, ARCHITECTURE, OS,
};

use super::{JavaDistribution, JavaPackage, JavaVersion};

const CHECK_INTERVAL: i64 = 7 * 24 * 3600;
const RETRY_INTERVAL: i64 = 3600;

fn cache(root: &Path, vendor: &JavaDistribution, major: u32) -> PathBuf {
    root.join(format!(
        "{}_{}_{}_{}",
        vendor.get_name(),
        major,
        OS,
        *ARCHITECTURE
    ))
}

async fn binary(dir: &Path) -> Result<PathBuf> {
    let mut files = fs::read_dir(dir).await?;
    while let Some(entry) = files.next_entry().await? {
        let folder_path = entry.path();
        let java_binary = match OS {
            OperatingSystem::WINDOWS => folder_path.join("bin").join("javaw.exe"),
            OperatingSystem::OSX => folder_path
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java"),
            _ => folder_path.join("bin").join("java"),
        };

        if java_binary.is_file() {
            if let Some(arch) = runtime_architecture(&java_binary).await {
                ensure!(arch == *ARCHITECTURE, "Cached Java is for {arch}");
            }
            return Ok(java_binary.absolutize()?.to_path_buf());
        }
    }

    Err(anyhow::anyhow!("Failed to find JRE"))
}

pub async fn find_cached_java_binary(
    root: &Path,
    vendor: &JavaDistribution,
    major: &u32,
) -> Result<PathBuf> {
    let dir = cache(root, vendor, *major);
    if let Ok(active) = fs::read_to_string(dir.join("active")).await {
        for hash in active.lines().filter(|s| valid_hash(s)) {
            if let Ok(java) = binary(&dir.join(hash)).await {
                return Ok(java);
            }
        }
    }
    binary(&root.join(format!("{}_{}", vendor.get_name(), major))).await
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
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

async fn lock_file(dir: &Path) -> Result<File> {
    fs::create_dir_all(dir).await?;
    Ok(fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("update.lock"))
        .await?
        .into_std()
        .await)
}

pub fn check_for_update(root: &Path, vendor: JavaDistribution, major: u32) {
    let root = root.to_owned();
    tokio::spawn(async move {
        if let Err(error) = update_if_due(&root, &vendor, major).await {
            warn!(
                "Could not update {} Java {major}: {error:#}",
                vendor.get_name()
            );
        }
    });
}

async fn update_if_due(root: &Path, vendor: &JavaDistribution, major: u32) -> Result<()> {
    let dir = cache(root, vendor, major);
    let lock = lock_file(&dir).await?;
    match lock.try_lock() {
        Ok(()) => (),
        Err(std::fs::TryLockError::WouldBlock) => return Ok(()),
        Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
    }
    let deadline = dir.join("next-check");
    let now = Utc::now().timestamp();
    let next = fs::read_to_string(&deadline)
        .await
        .ok()
        .and_then(|s| s.parse::<i64>().ok());
    if next.is_some_and(|next| next > now && next <= now + CHECK_INTERVAL) {
        return Ok(());
    }
    fs::write(&deadline, (now + RETRY_INTERVAL).to_string()).await?;
    let java = find_cached_java_binary(root, vendor, &major).await?;
    let package = vendor
        .package(&major)
        .await?
        .context("No matching Java package")?;
    if !java.starts_with(dir.join(&package.sha256)) {
        let installed = running_version(&java).await?;
        if package.version.is_newer_than(&installed) {
            install(&dir, major, &package, Some(&installed), |_, _| ()).await?;
        }
    }
    fs::write(deadline, (now + CHECK_INTERVAL).to_string()).await?;
    Ok(())
}

async fn running_version(java: &Path) -> Result<JavaVersion> {
    let probe = if cfg!(windows) {
        java.with_file_name("java.exe")
    } else {
        java.to_owned()
    };
    let output = tokio::time::timeout(
        Duration::from_secs(15),
        Command::new(probe)
            .args(["-Xmx32m", "-XshowSettings:properties", "-version"])
            .kill_on_drop(true)
            .output(),
    )
    .await??;
    ensure!(
        output.status.success(),
        "Java failed its startup check: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("java.runtime.version = ")
                .and_then(JavaVersion::parse)
        })
        .context("Java did not report a recognized runtime version")
}

pub async fn jre_download<F: Fn(u64, u64)>(
    root: &Path,
    vendor: &JavaDistribution,
    major: &u32,
    progress: F,
) -> Result<PathBuf> {
    let dir = cache(root, vendor, *major);
    let lock = lock_file(&dir).await?;
    let _lock = tokio::task::spawn_blocking(move || lock.lock().map(|_| lock)).await??;
    if let Ok(java) = find_cached_java_binary(root, vendor, major).await {
        return Ok(java);
    }
    let package = vendor
        .package(major)
        .await?
        .context("No matching Java package")?;
    let java = install(&dir, *major, &package, None, progress).await?;
    let _ = fs::write(
        dir.join("next-check"),
        (Utc::now().timestamp() + CHECK_INTERVAL).to_string(),
    )
    .await;
    Ok(java)
}

async fn install(
    dir: &Path,
    major: u32,
    package: &JavaPackage,
    installed: Option<&JavaVersion>,
    progress: impl Fn(u64, u64),
) -> Result<PathBuf> {
    let bytes = download_file(&package.url, progress).await?;
    activate(dir, major, package, installed, &bytes).await
}

async fn activate(
    dir: &Path,
    major: u32,
    package: &JavaPackage,
    installed: Option<&JavaVersion>,
    bytes: &[u8],
) -> Result<PathBuf> {
    ensure!(valid_hash(&package.sha256), "Invalid Java package checksum");
    ensure!(
        base16ct::lower::encode_string(&Sha256::digest(bytes))
            .eq_ignore_ascii_case(&package.sha256),
        "Java package checksum mismatch"
    );
    let stage = dir.join("partial");
    match fs::remove_dir_all(&stage).await {
        Err(e) if e.kind() != ErrorKind::NotFound => return Err(e.into()),
        _ => (),
    }
    fs::create_dir(&stage).await?;
    let archive = Cursor::new(bytes);
    match OS {
        OperatingSystem::WINDOWS => zip_extract(archive, &stage).await?,
        OperatingSystem::LINUX | OperatingSystem::OSX => tar_gz_extract(archive, &stage).await?,
        _ => bail!("Unsupported OS"),
    }
    let java = binary(&stage).await?;
    let version = running_version(&java).await?;
    ensure!(
        version.major() == major && package.version.matches(&version),
        "Downloaded Java does not match the requested release"
    );
    ensure!(
        installed.is_none_or(|old| version.is_newer_than(old)),
        "Refusing to downgrade Java"
    );

    // A game can still be using any previously selected directory.
    let dest = dir.join(&package.sha256);
    if dest.exists() {
        // An interrupted activation can leave a fully validated directory here.
        binary(&dest).await?;
        fs::remove_dir_all(stage).await?;
    } else {
        fs::rename(stage, &dest).await?;
    }
    let previous = fs::read_to_string(dir.join("active"))
        .await
        .unwrap_or_default();
    let active = previous
        .lines()
        .find(|hash| valid_hash(hash) && *hash != package.sha256)
        .map_or_else(
            || package.sha256.clone(),
            |old| format!("{}\n{old}", package.sha256),
        );
    fs::write(dir.join("active.tmp"), active).await?;
    fs::rename(dir.join("active.tmp"), dir.join("active")).await?;
    binary(&dest).await
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use async_compression::tokio::write::GzipEncoder;
    use tokio::io::AsyncWriteExt;

    async fn archive(version: &str) -> Result<(JavaPackage, Vec<u8>)> {
        let script = format!("#!/bin/sh\necho 'java.runtime.version = {version}' >&2\n");
        let mut tar = tokio_tar::Builder::new(GzipEncoder::new(Vec::new()));
        let mut header = tokio_tar::Header::new_gnu();
        header.set_size(script.len() as u64);
        header.set_mode(0o755);
        tar.append_data(&mut header, "jdk/bin/java", script.as_bytes())
            .await?;
        let mut gzip = tar.into_inner().await?;
        gzip.shutdown().await?;
        let bytes = gzip.into_inner();
        Ok((
            JavaPackage {
                url: String::new(),
                sha256: base16ct::lower::encode_string(&Sha256::digest(&bytes)),
                version: JavaVersion::parse(version).unwrap(),
            },
            bytes,
        ))
    }

    #[tokio::test]
    async fn selects_only_completed_upgrades() -> Result<()> {
        let root = tempfile::tempdir()?;
        let vendor = JavaDistribution::Temurin;
        let dir = cache(root.path(), &vendor, 21);
        let lock = lock_file(&dir).await?;
        lock.lock()?;
        let (old, bytes) = archive("21.0.8+9").await?;
        let first = activate(&dir, 21, &old, None, &bytes).await?;
        let (new, bytes) = archive("21.0.9+10").await?;
        fs::create_dir_all(dir.join("partial/jdk/bin")).await?;
        update_if_due(root.path(), &vendor, 21).await?;
        assert!(!dir.join("next-check").exists());
        assert!(activate(&dir, 21, &new, Some(&old.version), b"truncated")
            .await
            .is_err());
        assert!(activate(&dir, 25, &new, None, &bytes).await.is_err());
        assert_eq!(
            find_cached_java_binary(root.path(), &vendor, &21).await?,
            first
        );
        let next = activate(&dir, 21, &new, Some(&old.version), &bytes).await?;
        assert_eq!(
            find_cached_java_binary(root.path(), &vendor, &21).await?,
            next
        );
        assert_eq!(running_version(&first).await?, old.version);
        drop(lock);
        let deadline = (Utc::now().timestamp() + CHECK_INTERVAL).to_string();
        fs::write(dir.join("next-check"), &deadline).await?;
        update_if_due(root.path(), &vendor, 21).await?;
        assert_eq!(fs::read_to_string(dir.join("next-check")).await?, deadline);
        fs::remove_file(next).await?;
        assert_eq!(
            find_cached_java_binary(root.path(), &vendor, &21).await?,
            first
        );
        Ok(())
    }
}
