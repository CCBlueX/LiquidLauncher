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

//! The Java installations the launcher manages, in the `runtimes` folder of its data directory.
//!
//! ```text
//! temurin_21_linux_x64/     a cache: one per vendor, Java version and platform
//!   active                  checksum of the installation that launches use
//!   last-used               when a launch last picked the cache
//!   update.lock             held while a package is installed or old installations are removed
//!   <sha256>/               an installation: one unpacked package
//!     in-use                locked by every game running from it
//!     jdk-21.0.13+11-jre/
//! temurin_21/               an installation by a launcher from before the caches
//! ```

use std::fs::File;
use std::io::{self, Cursor, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use base16ct::lower::encode_string;
use chrono::Utc;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
use tokio::{fs, process::Command, task::spawn_blocking, time::timeout};
use tracing::warn;

use crate::utils::{download_file, tar_gz_extract, zip_extract, Architecture, ARCHITECTURE, OS};

use super::{JavaDistribution, JavaPackage, JavaRuntime, JavaVersion};

/// How long a folder that no launch picks stays on disk.
const STALE_AFTER: Duration = Duration::from_secs(30 * 24 * 60 * 60);

fn cache_dir(root: &Path, vendor: &JavaDistribution, major: u32) -> PathBuf {
    root.join(format!("{}_{major}_{OS}_{}", vendor.get_name(), *ARCHITECTURE))
}

fn legacy_dir(root: &Path, vendor: &JavaDistribution, major: u32) -> PathBuf {
    root.join(format!("{}_{major}", vendor.get_name()))
}

/// The installation that launches use for a vendor and Java version.
pub async fn find_java(root: &Path, vendor: &JavaDistribution, major: u32) -> Option<JavaRuntime> {
    let cache = cache_dir(root, vendor, major);
    if let Ok(active) = fs::read_to_string(cache.join("active")).await {
        if let Ok(java) = open(&cache.join(active)).await {
            mark_used(&cache).await;
            return Some(java);
        }
    }

    let legacy = legacy_dir(root, vendor, major);
    let java = open(&legacy).await.ok()?;
    mark_used(&legacy).await;
    Some(java)
}

/// An installation for a Java version, whatever its vendor.
pub async fn find_any_java(root: &Path, major: u32) -> Option<JavaRuntime> {
    let mut folders = fs::read_dir(root).await.ok()?;
    while let Ok(Some(folder)) = folders.next_entry().await {
        let name = folder.file_name();
        // Both a cache and a legacy folder are named `<vendor>_<major>`, a cache with more behind.
        let mut parts = name.to_str().unwrap_or_default().split('_');
        let (Some(vendor), Some(version)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Ok(vendor) = JavaDistribution::try_from(vendor.to_owned()) else {
            continue;
        };
        if version == major.to_string() {
            if let Some(java) = find_java(root, &vendor, major).await {
                return Some(java);
            }
        }
    }
    None
}

async fn open(installation: &Path) -> Result<JavaRuntime> {
    let java = java_binary(installation).await?;
    Ok(JavaRuntime::leased(java, lease(installation).await?))
}

/// The Java executable of an installation, which holds one unpacked JDK or JRE.
async fn java_binary(installation: &Path) -> Result<PathBuf> {
    let mut entries = fs::read_dir(installation).await?;
    while let Some(entry) = entries.next_entry().await? {
        let home = if cfg!(target_os = "macos") {
            entry.path().join("Contents/Home")
        } else {
            entry.path()
        };
        let java = home
            .join("bin")
            .join(if cfg!(windows) { "javaw.exe" } else { "java" });
        if !fs::metadata(&java).await.is_ok_and(|file| file.is_file()) {
            continue;
        }
        // Launchers that ran emulated (Rosetta, Windows on ARM) left x64 Java in the legacy folders.
        if let Some(architecture) = runtime_architecture(&home).await {
            ensure!(
                architecture == *ARCHITECTURE,
                "The installed Java is for {architecture}"
            );
        }
        return Ok(java.absolutize()?.to_path_buf());
    }
    bail!("No Java in {}", installation.display())
}

/// From the `release` file of a Java home, if it names a known architecture.
async fn runtime_architecture(home: &Path) -> Option<Architecture> {
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

/// A shared lock on an installation. The game started with it keeps the installation from being
/// deleted, also after the launcher has quit.
async fn lease(installation: &Path) -> io::Result<File> {
    let in_use = installation.join("in-use");
    // An installation by an older launcher has no such file yet.
    let _ = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&in_use)
        .await;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    // On Windows the game inherits the open file but not the lock. Sharing the file for reading
    // only (FILE_SHARE_READ) is what refuses `remove_unless_running` there.
    #[cfg(windows)]
    options.share_mode(1);
    let file = options.open(in_use).await?.into_std().await;
    file.try_lock_shared()?;
    Ok(file)
}

/// Installs a package and makes it what launches use, unless the installed Java is as new
/// already.
pub async fn install_java<F: Fn(u64, u64)>(
    root: &Path,
    package: &JavaPackage,
    progress: F,
) -> Result<JavaRuntime> {
    let cache = cache_dir(root, &package.vendor, package.major);
    let _lock = lock_updates(&cache).await?;

    if let Some(java) = find_java(root, &package.vendor, package.major).await {
        if java.path().starts_with(cache.join(&package.sha256)) {
            return Ok(java);
        }
        // Another package, or the Java of an older launcher: its version decides.
        let installed = running_version(java.path()).await?;
        if !package.version.is_newer_than(&installed) {
            return Ok(java);
        }
    }

    let archive = download_file(&package.url, progress).await?;
    activate(&cache, package, &archive).await?;
    find_java(root, &package.vendor, package.major)
        .await
        .context("The installed Java cannot be used")
}

/// Unpacks a downloaded package and makes it the installation that launches use.
async fn activate(cache: &Path, package: &JavaPackage, archive: &[u8]) -> Result<()> {
    ensure!(
        encode_string(&Sha256::digest(archive)).eq_ignore_ascii_case(&package.sha256),
        "Java package checksum mismatch"
    );

    let staged = cache.join("partial");
    match fs::remove_dir_all(&staged).await {
        Err(error) if error.kind() != ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    fs::create_dir(&staged).await?;
    match package.archive.as_str() {
        "zip" => zip_extract(Cursor::new(archive), &staged).await?,
        "tar.gz" => tar_gz_extract(Cursor::new(archive), &staged).await?,
        other => bail!("Unsupported Java archive type {other}"),
    }
    let unpacked = running_version(&java_binary(&staged).await?).await?;
    ensure!(
        package.version.matches(&unpacked),
        "Downloaded Java is not the release its package names"
    );

    let installation = cache.join(&package.sha256);
    // Only there when an earlier installation of this package lost files.
    ensure!(
        remove_unless_running(&installation).await?,
        "A running game uses the installation to replace"
    );
    fs::rename(&staged, &installation).await?;

    // Renamed into place, so a launch reads either the old checksum or the new one.
    fs::write(cache.join("active.tmp"), &package.sha256).await?;
    fs::rename(cache.join("active.tmp"), cache.join("active")).await?;
    Ok(())
}

/// The version a Java executable reports when run.
async fn running_version(java: &Path) -> Result<JavaVersion> {
    // javaw.exe has no console to report to.
    let java = if cfg!(windows) {
        java.with_file_name("java.exe")
    } else {
        java.to_owned()
    };
    let mut command = Command::new(java);
    command
        .args(["-Xmx32m", "-XshowSettings:properties", "-version"])
        .kill_on_drop(true);
    // CREATE_NO_WINDOW: java.exe would open a console window next to the launcher.
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);

    let output = timeout(Duration::from_secs(15), command.output()).await??;
    let report = String::from_utf8_lossy(&output.stderr);
    ensure!(
        output.status.success(),
        "Java failed its startup check: {report}"
    );
    report
        .lines()
        .find_map(|line| JavaVersion::parse(line.trim().strip_prefix("java.runtime.version = ")?))
        .context("Java did not report a recognized runtime version")
}

/// Waits for its turn to change a cache: installing a package and removing old installations
/// keep each other out, across launchers too.
async fn lock_updates(cache: &Path) -> Result<File> {
    fs::create_dir_all(cache).await?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(cache.join("update.lock"))
        .await?
        .into_std()
        .await;
    Ok(spawn_blocking(move || lock.lock().map(|_| lock)).await??)
}

/// Deletes what launches no longer need: installations a newer package replaced, and folders
/// that no launch picked for [`STALE_AFTER`]. What a game still runs from stays for a later call.
pub async fn remove_old_java(root: &Path) -> Result<()> {
    let mut folders = fs::read_dir(root).await?;
    while let Some(folder) = folders.next_entry().await? {
        let folder = folder.path();
        if !fs::metadata(&folder).await?.is_dir() {
            continue;
        }
        let cleaned = if fs::try_exists(folder.join("update.lock")).await? {
            clean_cache(&folder).await
        } else {
            clean_legacy(root, &folder).await
        };
        if let Err(error) = cleaned {
            warn!("Could not clean up {}: {error}", folder.display());
        }
    }
    Ok(())
}

async fn clean_cache(cache: &Path) -> Result<()> {
    let lock = lock_updates(cache).await?;

    let stale = is_stale(cache).await;
    let active = fs::read_to_string(cache.join("active"))
        .await
        .unwrap_or_default();
    let mut is_running = false;
    let mut entries = fs::read_dir(cache).await?;
    while let Some(entry) = entries.next_entry().await? {
        // Every folder is an installation, or what an interrupted one left behind.
        let is_replaced = entry.file_name().to_str() != Some(active.as_str());
        if entry.file_type().await?.is_dir() && (stale || is_replaced) {
            is_running |= !remove_unless_running(&entry.path()).await?;
        }
    }

    if stale && !is_running {
        drop(lock);
        fs::remove_dir_all(cache).await?;
    }
    Ok(())
}

/// A folder from before the caches is one installation, replaced once its cache has one.
async fn clean_legacy(root: &Path, legacy: &Path) -> Result<()> {
    let name = legacy.file_name().unwrap_or_default().to_string_lossy();
    let cache = root.join(format!("{name}_{OS}_{}", *ARCHITECTURE));
    if fs::try_exists(cache.join("active")).await? || is_stale(legacy).await {
        remove_unless_running(legacy).await?;
    }
    Ok(())
}

/// Deletes an installation, or answers `false` while a game runs from it.
async fn remove_unless_running(installation: &Path) -> Result<bool> {
    let opened = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(installation.join("in-use"))
        .await;
    let lease = match opened {
        Ok(file) => file.into_std().await,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(true),
        // Windows refuses to open the file for writing while a game has it, see `lease`.
        Err(_) => return Ok(false),
    };
    if lease.try_lock().is_err() {
        return Ok(false);
    }

    // The locked file goes last, so no launch can pick what is half deleted.
    let mut entries = fs::read_dir(installation).await?;
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
    fs::remove_dir_all(installation).await?;
    Ok(true)
}

async fn mark_used(folder: &Path) {
    let now = Utc::now().timestamp().to_string();
    let _ = fs::write(folder.join("last-used"), now).await;
}

/// Whether no launch picked the folder for [`STALE_AFTER`]. One without a record waits from now.
async fn is_stale(folder: &Path) -> bool {
    let last_used = fs::read_to_string(folder.join("last-used"))
        .await
        .ok()
        .and_then(|at| at.trim().parse::<i64>().ok());
    match last_used {
        Some(at) => Utc::now().timestamp() - at > STALE_AFTER.as_secs() as i64,
        None => {
            mark_used(folder).await;
            false
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use async_compression::tokio::write::GzipEncoder;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Child;
    use tokio::sync::Mutex;

    /// A process started while another test unpacks its `java` keeps that file open for a
    /// moment, and running it then fails as "text file busy".
    static ONE_AT_A_TIME: Mutex<()> = Mutex::const_new(());

    /// A package whose `java` reports the version, and sleeps when told to `hold`.
    async fn package(version: &str) -> Result<(JavaPackage, Vec<u8>)> {
        let script = format!("#!/bin/sh\nif [ \"$1\" = hold ]; then exec sleep 60; fi\necho 'java.runtime.version = {version}' >&2\n");
        let mut tar = tokio_tar::Builder::new(GzipEncoder::new(Vec::new()));
        let mut header = tokio_tar::Header::new_gnu();
        header.set_size(script.len() as u64);
        header.set_mode(0o755);
        tar.append_data(&mut header, "jdk/bin/java", script.as_bytes())
            .await?;
        let mut gzip = tar.into_inner().await?;
        gzip.shutdown().await?;
        let archive = gzip.into_inner();
        let package = JavaPackage {
            major: 21,
            vendor: JavaDistribution::try_from("example-jdk".to_owned()).unwrap(),
            os: "linux".into(),
            arch: ARCHITECTURE.to_string(),
            archive: "tar.gz".into(),
            url: String::new(),
            sha256: encode_string(&Sha256::digest(&archive)),
            version: JavaVersion::parse(version).unwrap(),
        };
        Ok((package, archive))
    }

    async fn new_cache(root: &Path, package: &JavaPackage) -> Result<PathBuf> {
        let cache = cache_dir(root, &package.vendor, package.major);
        lock_updates(&cache).await?;
        Ok(cache)
    }

    /// A game that runs from the installation until it is killed.
    async fn play(java: JavaRuntime, root: &Path) -> Result<Child> {
        java.execute(vec!["hold".into()], root).await
    }

    #[tokio::test]
    async fn finds_any_installation_of_the_java_version() -> Result<()> {
        let root = tempfile::tempdir()?;
        let java = root.path().join("unlisted-jdk_21/jdk/bin/java");
        fs::create_dir_all(java.parent().unwrap()).await?;
        fs::write(&java, []).await?;

        assert!(find_any_java(root.path(), 17).await.is_none());
        assert_eq!(find_any_java(root.path(), 21).await.unwrap().path(), java);
        Ok(())
    }

    #[tokio::test]
    async fn a_newer_package_replaces_the_installation_once_no_game_runs_from_it() -> Result<()> {
        let root = tempfile::tempdir()?;
        let root = root.path();
        let _alone = ONE_AT_A_TIME.lock().await;
        let (old, archive) = package("21.0.8+9").await?;
        let cache = new_cache(root, &old).await?;
        let find = || async { find_java(root, &old.vendor, 21).await };

        activate(&cache, &old, &archive).await?;
        let first = cache.join(&old.sha256);
        let mut game = play(find().await.unwrap(), root).await?;

        let (new, archive) = package("21.0.9+10").await?;
        assert!(activate(&cache, &new, b"truncated").await.is_err());
        let mislabelled = JavaPackage {
            version: JavaVersion::parse("21.0.10+1").unwrap(),
            ..new.clone()
        };
        assert!(activate(&cache, &mislabelled, &archive).await.is_err());
        assert!(find().await.unwrap().path().starts_with(&first));

        activate(&cache, &new, &archive).await?;
        let second = cache.join(&new.sha256);
        assert!(find().await.unwrap().path().starts_with(&second));

        remove_old_java(root).await?;
        assert_eq!(
            running_version(&first.join("jdk/bin/java")).await?,
            old.version
        );
        game.kill().await?;
        remove_old_java(root).await?;
        assert!(!fs::try_exists(&first).await?);

        fs::remove_file(second.join("jdk/bin/java")).await?;
        assert!(find().await.is_none());
        activate(&cache, &new, &archive).await?;
        assert!(find().await.unwrap().path().starts_with(&second));
        Ok(())
    }

    #[tokio::test]
    async fn an_installation_by_an_older_launcher_goes_once_replaced() -> Result<()> {
        let root = tempfile::tempdir()?;
        let root = root.path();
        let _alone = ONE_AT_A_TIME.lock().await;
        let (package, archive) = package("21.0.9+10").await?;
        let legacy = legacy_dir(root, &package.vendor, 21);
        fs::create_dir(&legacy).await?;
        tar_gz_extract(Cursor::new(&archive[..]), &legacy).await?;

        let java = find_java(root, &package.vendor, 21).await.unwrap();
        assert!(java.path().starts_with(&legacy));
        let mut game = play(java, root).await?;
        remove_old_java(root).await?;
        assert!(fs::try_exists(&legacy).await?);

        let cache = new_cache(root, &package).await?;
        activate(&cache, &package, &archive).await?;
        let java = find_java(root, &package.vendor, 21).await.unwrap();
        assert!(java.path().starts_with(&cache));
        remove_old_java(root).await?;
        assert!(fs::try_exists(legacy.join("jdk/bin/java")).await?);

        game.kill().await?;
        remove_old_java(root).await?;
        assert!(!fs::try_exists(&legacy).await?);
        Ok(())
    }

    #[tokio::test]
    async fn what_no_launch_picked_for_a_month_goes() -> Result<()> {
        let root = tempfile::tempdir()?;
        let root = root.path();
        let _alone = ONE_AT_A_TIME.lock().await;
        let (package, archive) = package("21.0.9+10").await?;
        let cache = new_cache(root, &package).await?;
        activate(&cache, &package, &archive).await?;
        let java = find_java(root, &package.vendor, 21).await.unwrap();
        let mut game = play(java, root).await?;

        let long_unused = root.join("17");
        fs::create_dir_all(long_unused.join("jdk/bin")).await?;
        fs::write(long_unused.join("last-used"), "1\n").await?;
        let never_seen = root.join("unseen-jdk_8");
        fs::create_dir(&never_seen).await?;

        remove_old_java(root).await?;
        assert!(!fs::try_exists(&long_unused).await?);
        assert!(fs::try_exists(never_seen.join("last-used")).await?);
        assert!(fs::try_exists(&cache).await?);

        fs::write(cache.join("last-used"), "1").await?;
        remove_old_java(root).await?;
        assert!(fs::try_exists(cache.join(&package.sha256).join("jdk/bin/java")).await?);
        game.kill().await?;
        remove_old_java(root).await?;
        assert!(!fs::try_exists(&cache).await?);
        Ok(())
    }
}
