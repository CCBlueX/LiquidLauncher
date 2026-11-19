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
use std::io::{self, Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use base16ct::lower::encode_string;
use chrono::Utc;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
use tokio::{fs, process::Command, spawn, task::spawn_blocking, time::timeout};
use tracing::warn;

use crate::utils::{
    download_file, tar_gz_extract, zip_extract, Architecture, OperatingSystem, ARCHITECTURE, OS,
};

use super::{JavaDistribution, JavaPackage, JavaRuntime, JavaVersion};

/// How long an installation that no launch picks stays on disk.
const STALE_AFTER: Duration = Duration::from_secs(30 * 24 * 60 * 60);

fn cache(root: &Path, vendor: &JavaDistribution, major: u32) -> PathBuf {
    root.join(format!(
        "{}_{}_{}_{}",
        vendor.get_name(),
        major,
        OS,
        *ARCHITECTURE
    ))
}

/// Where launchers before the per-platform caches kept the one installation of a vendor and major.
fn legacy_cache(root: &Path, vendor: &JavaDistribution, major: u32) -> PathBuf {
    root.join(format!("{}_{}", vendor.get_name(), major))
}

/// A shared hold on an installation. The game inherits it, so what it runs from is not deleted.
async fn lease(dir: &Path) -> io::Result<Option<File>> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    options.share_mode(1); // FILE_SHARE_READ survives handle inheritance.
    match options.open(dir.join("in-use")).await {
        Ok(file) => {
            let file = file.into_std().await;
            file.try_lock_shared()?;
            Ok(Some(file))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

async fn runtime(dir: &Path, lease: Option<File>) -> Result<JavaRuntime> {
    let mut files = fs::read_dir(dir).await?;
    while let Some(entry) = files.next_entry().await? {
        let home = if cfg!(target_os = "macos") {
            entry.path().join("Contents/Home")
        } else {
            entry.path()
        };
        let java_binary = home
            .join("bin")
            .join(if cfg!(windows) { "javaw.exe" } else { "java" });

        if fs::metadata(&java_binary).await.is_ok_and(|m| m.is_file()) {
            let release = fs::read_to_string(home.join("release"))
                .await
                .unwrap_or_default();
            if let Some(arch) = runtime_architecture(&release) {
                ensure!(arch == *ARCHITECTURE, "Cached Java is for {arch}");
            }
            let version = ["JAVA_RUNTIME_VERSION=", "JAVA_VERSION="]
                .into_iter()
                .find_map(|key| {
                    release.lines().find_map(|line| {
                        JavaVersion::parse(line.strip_prefix(key)?.trim_matches('"'))
                    })
                });
            return Ok(JavaRuntime::cached(
                java_binary.absolutize()?.to_path_buf(),
                lease,
                version,
            ));
        }
    }

    Err(anyhow::anyhow!("Failed to find JRE"))
}

pub async fn find_cached_java_binary(
    root: &Path,
    vendor: &JavaDistribution,
    major: &u32,
) -> Result<JavaRuntime> {
    let dir = cache(root, vendor, *major);
    if fs::try_exists(dir.join("prune-pending"))
        .await
        .unwrap_or(false)
    {
        let dir = dir.clone();
        spawn(async move {
            if let Ok(lock) = lock_file(&dir).await {
                if lock.try_lock().is_ok() {
                    if let Err(error) = prune(&dir).await {
                        warn!("Could not prune Java cache: {error}");
                    }
                }
            }
        });
    }
    if let Ok(active) = fs::read_to_string(dir.join("active")).await {
        for hash in active.lines().filter(|s| valid_hash(s)) {
            let generation = dir.join(hash);
            let Ok(held) = lease(&generation).await else {
                continue;
            };
            if let Ok(java) = runtime(&generation, held).await {
                mark_used(&dir).await;
                let legacy = legacy_cache(root, vendor, *major);
                spawn(async move {
                    if let Err(error) = remove_unused(&legacy).await {
                        warn!("Could not remove replaced Java installation: {error}");
                    }
                });
                return Ok(java);
            }
        }
    }
    let legacy = legacy_cache(root, vendor, *major);
    // Older launchers wrote no lease file.
    let _ = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(legacy.join("in-use"))
        .await;
    let held = lease(&legacy).await?;
    let java = runtime(&legacy, held).await?;
    mark_used(&legacy).await;
    Ok(java)
}

pub async fn find_unlisted_java(
    root: &Path,
    major: u32,
    listed: impl Fn(&str) -> bool,
) -> Option<JavaRuntime> {
    let mut entries = fs::read_dir(root).await.ok()?;
    let legacy = major.to_string();
    let platform = format!("{major}_{OS}_{}", *ARCHITECTURE);
    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(mut name) = entry.file_name().into_string() else { continue };
        let Some((vendor, suffix)) = name.split_once('_') else { continue };
        if (suffix != legacy && suffix != platform) || listed(vendor) {
            continue;
        }
        name.truncate(vendor.len());
        if let Ok(vendor) = JavaDistribution::try_from(name) {
            if let Ok(runtime) = find_cached_java_binary(root, &vendor, &major).await {
                return Some(runtime);
            }
        }
    }
    None
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn runtime_architecture(release: &str) -> Option<Architecture> {
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

/// Deletes an installation, or answers `false` while a game still runs from it.
async fn remove_unused(dir: &Path) -> Result<bool> {
    let lease = match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("in-use"))
        .await
    {
        Ok(file) => file.into_std().await,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(true),
        // Windows refuses the open while a game holds the lease.
        Err(_) => return Ok(false),
    };
    if lease.try_lock().is_err() {
        return Ok(false);
    }
    // The lease goes last: until then nobody can start a game from what is left.
    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_name() == "in-use" {
            continue;
        }
        if entry.file_type().await?.is_dir() {
            fs::remove_dir_all(entry.path()).await?;
        } else {
            fs::remove_file(entry.path()).await?;
        }
    }
    drop(lease);
    fs::remove_dir_all(dir).await?;
    Ok(true)
}

async fn prune(dir: &Path) -> Result<()> {
    let active = fs::read_to_string(dir.join("active")).await?;
    let mut entries = fs::read_dir(dir).await?;
    let mut pending = false;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let Some(hash) = name.to_str().filter(|name| valid_hash(name)) else {
            continue;
        };
        if active.lines().any(|kept| hash == kept) || !entry.file_type().await?.is_dir() {
            continue;
        }
        pending |= !remove_unused(&entry.path()).await?;
    }
    if !pending {
        fs::remove_file(dir.join("prune-pending")).await?;
    }
    Ok(())
}

async fn mark_used(dir: &Path) {
    let _ = fs::write(dir.join("last-used"), Utc::now().timestamp().to_string()).await;
}

/// Deletes the installations no launch picked for [`STALE_AFTER`], whichever launcher version
/// made them. One seen for the first time counts as used now.
pub async fn remove_stale(root: &Path) -> Result<()> {
    let mut entries = fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        let dir = entry.path();
        if !entry.file_type().await?.is_dir() {
            continue;
        }
        let used = fs::read_to_string(dir.join("last-used"))
            .await
            .ok()
            .and_then(|at| at.trim().parse::<i64>().ok());
        match used {
            None => mark_used(&dir).await,
            Some(at) if Utc::now().timestamp() - at <= STALE_AFTER.as_secs() as i64 => (),
            Some(_) => {
                if let Err(error) = remove_cache(&dir).await {
                    warn!("Could not remove unused Java installation: {error}");
                }
            }
        }
    }
    Ok(())
}

/// Deletes a cache with every installation in it that no game runs from.
async fn remove_cache(dir: &Path) -> Result<()> {
    let lock = lock_file(dir).await?;
    if lock.try_lock().is_err() {
        return Ok(());
    }
    let mut in_use = false;
    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_name().to_str().is_some_and(valid_hash) && entry.file_type().await?.is_dir() {
            in_use |= !remove_unused(&entry.path()).await?;
        }
    }
    drop(lock);
    if !in_use {
        remove_unused(dir).await?;
    }
    Ok(())
}

async fn running_version(java: &Path) -> Result<JavaVersion> {
    let probe = if cfg!(windows) {
        java.with_file_name("java.exe")
    } else {
        java.to_owned()
    };
    let output = timeout(
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
    package: &JavaPackage,
    progress: F,
) -> Result<JavaRuntime> {
    let major = package.major;
    let vendor = &package.vendor;
    let dir = cache(root, vendor, major);
    let lock = lock_file(&dir).await?;
    let _lock = spawn_blocking(move || lock.lock().map(|_| lock)).await??;
    let mut installed = None;
    if let Ok(java) = find_cached_java_binary(root, vendor, &major).await {
        if java.path().starts_with(dir.join(&package.sha256)) {
            return Ok(java);
        }
        let version = running_version(java.path()).await?;
        if !package.version.is_newer_than(&version) {
            return Ok(java);
        }
        installed = Some(version);
    }
    let bytes = download_file(&package.url, progress).await?;
    activate(&dir, major, package, installed.as_ref(), &bytes).await?;
    find_cached_java_binary(root, vendor, &major).await
}

async fn activate(
    dir: &Path,
    major: u32,
    package: &JavaPackage,
    installed: Option<&JavaVersion>,
    bytes: &[u8],
) -> Result<()> {
    ensure!(valid_hash(&package.sha256), "Invalid Java package checksum");
    ensure!(
        encode_string(&Sha256::digest(bytes)).eq_ignore_ascii_case(&package.sha256),
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
    let java = runtime(&stage, None).await?;
    let version = running_version(java.path()).await?;
    ensure!(
        version.major() == major && package.version.matches(&version),
        "Downloaded Java does not match the requested release"
    );
    ensure!(
        installed.is_none_or(|old| version.is_newer_than(old)),
        "Refusing to downgrade Java"
    );

    let dest = dir.join(&package.sha256);
    let intact = match runtime(&dest, None).await {
        Ok(existing) => running_version(existing.path())
            .await
            .is_ok_and(|found| found == version),
        Err(_) => false,
    };
    if intact {
        fs::remove_dir_all(stage).await?;
    } else {
        ensure!(
            remove_unused(&dest).await?,
            "The damaged Java installation is still in use"
        );
        fs::File::create_new(stage.join("in-use")).await?;
        fs::rename(stage, &dest).await?;
    }
    fs::write(dir.join("active.tmp"), &package.sha256).await?;
    fs::rename(dir.join("active.tmp"), dir.join("active")).await?;
    fs::write(dir.join("prune-pending"), b"").await?;
    if let Err(error) = prune(dir).await {
        warn!("Could not prune Java cache: {error}");
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use async_compression::tokio::write::GzipEncoder;
    use tokio::{io::AsyncWriteExt, time::sleep};

    async fn archive(version: &str) -> Result<(JavaPackage, Vec<u8>)> {
        let script = format!("#!/bin/sh\nif [ \"$1\" = hold ]; then exec sleep 60; fi\necho 'java.runtime.version = {version}' >&2\n");
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
                major: 21,
                vendor: JavaDistribution::try_from("example-jdk".to_owned()).unwrap(),
                os: "linux".into(),
                arch: ARCHITECTURE.to_string(),
                archive: "tar.gz".into(),
                url: String::new(),
                sha256: encode_string(&Sha256::digest(&bytes)),
                version: JavaVersion::parse(version).unwrap(),
            },
            bytes,
        ))
    }

    #[tokio::test]
    async fn discovers_only_matching_legacy_caches() -> Result<()> {
        let root = tempfile::tempdir()?;
        let bin = root.path().join("unlisted-jdk_21/jdk/bin");
        fs::create_dir_all(&bin).await?;
        let java = bin.join("java");
        fs::write(&java, []).await?;
        assert!(find_unlisted_java(root.path(), 17, |_| false).await.is_none());
        assert!(find_unlisted_java(root.path(), 21, |name| name == "unlisted-jdk").await.is_none());
        assert_eq!(find_unlisted_java(root.path(), 21, |_| false).await.unwrap().path(), java);
        Ok(())
    }

    #[tokio::test]
    async fn removes_a_replaced_legacy_installation_once_unused() -> Result<()> {
        let root = tempfile::tempdir()?;
        let (package, bytes) = archive("21.0.9+10").await?;
        let vendor = &package.vendor;
        let legacy = legacy_cache(root.path(), vendor, 21);
        fs::create_dir_all(&legacy).await?;
        tar_gz_extract(Cursor::new(&bytes[..]), &legacy).await?;
        let playing = find_cached_java_binary(root.path(), vendor, &21).await?;
        assert_eq!(playing.path(), legacy.join("jdk/bin/java"));
        let mut game = playing.execute(vec!["hold".into()], root.path()).await?;
        drop(playing);

        let dir = cache(root.path(), vendor, 21);
        let lock = lock_file(&dir).await?;
        lock.lock()?;
        activate(&dir, 21, &package, None, &bytes).await?;
        assert!(!remove_unused(&legacy).await?);
        assert!(fs::try_exists(legacy.join("jdk/bin/java")).await?);

        game.kill().await?;
        let managed = find_cached_java_binary(root.path(), vendor, &21).await?;
        assert!(managed.path().starts_with(&dir));
        timeout(Duration::from_secs(5), async {
            while fs::try_exists(&legacy).await.unwrap_or(true) {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn keeps_only_the_newest_completed_upgrade() -> Result<()> {
        let root = tempfile::tempdir()?;
        let (old, bytes) = archive("21.0.8+9").await?;
        let vendor = &old.vendor;
        let dir = cache(root.path(), vendor, 21);
        let lock = lock_file(&dir).await?;
        lock.lock()?;
        activate(&dir, 21, &old, None, &bytes).await?;
        let first = dir.join(&old.sha256).join("jdk/bin/java");
        let playing = find_cached_java_binary(root.path(), vendor, &21).await?;
        let mut game = playing.execute(vec!["hold".into()], root.path()).await?;
        drop(playing);
        let (new, bytes) = archive("21.0.9+10").await?;
        fs::create_dir_all(dir.join("partial/jdk/bin")).await?;
        assert!(activate(&dir, 21, &new, Some(&old.version), b"truncated")
            .await
            .is_err());
        assert!(activate(&dir, 25, &new, None, &bytes).await.is_err());
        assert_eq!(
            find_cached_java_binary(root.path(), vendor, &21)
                .await?
                .path(),
            first
        );
        activate(&dir, 21, &new, Some(&old.version), &bytes).await?;
        let next = dir.join(&new.sha256).join("jdk/bin/java");
        assert_eq!(
            find_cached_java_binary(root.path(), vendor, &21)
                .await?
                .path(),
            next
        );
        assert_eq!(running_version(&first).await?, old.version);
        game.kill().await?;
        prune(&dir).await?;
        assert!(!fs::try_exists(dir.join(&old.sha256)).await?);

        fs::remove_file(&next).await?;
        assert!(find_cached_java_binary(root.path(), vendor, &21)
            .await
            .is_err());
        activate(&dir, 21, &new, None, &bytes).await?;
        assert_eq!(
            find_cached_java_binary(root.path(), vendor, &21)
                .await?
                .path(),
            next
        );
        Ok(())
    }

    #[tokio::test]
    async fn removes_what_no_launch_used_for_a_month() -> Result<()> {
        let root = tempfile::tempdir()?;
        let (package, bytes) = archive("21.0.9+10").await?;
        let vendor = &package.vendor;
        let dir = cache(root.path(), vendor, 21);
        drop(lock_file(&dir).await?);
        activate(&dir, 21, &package, None, &bytes).await?;
        let playing = find_cached_java_binary(root.path(), vendor, &21).await?;
        let mut game = playing.execute(vec!["hold".into()], root.path()).await?;
        drop(playing);
        let ancient = root.path().join("17");
        fs::create_dir_all(ancient.join("jdk/bin")).await?;
        fs::write(ancient.join("last-used"), "1\n").await?;
        let unseen = root.path().join("unseen-jdk_8");
        fs::create_dir(&unseen).await?;

        remove_stale(root.path()).await?;
        assert!(!fs::try_exists(&ancient).await?);
        assert!(fs::try_exists(unseen.join("last-used")).await?);
        assert!(fs::try_exists(&dir).await?);

        fs::write(dir.join("last-used"), "1").await?;
        remove_stale(root.path()).await?;
        assert!(fs::try_exists(dir.join(&package.sha256).join("jdk/bin/java")).await?);
        game.kill().await?;
        remove_stale(root.path()).await?;
        assert!(!fs::try_exists(&dir).await?);
        Ok(())
    }
}
