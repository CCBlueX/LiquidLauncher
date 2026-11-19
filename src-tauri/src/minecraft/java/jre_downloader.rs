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
use base16ct::lower::encode_string;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
use tokio::{fs, process::Command, spawn, task::spawn_blocking, time::timeout};
use tracing::warn;

use crate::utils::{
    download_file, tar_gz_extract, zip_extract, Architecture, OperatingSystem, ARCHITECTURE, OS,
};

use super::{JavaDistribution, JavaPackage, JavaRuntime, JavaVersion};

fn cache(root: &Path, vendor: &JavaDistribution, major: u32) -> PathBuf {
    root.join(format!(
        "{}_{}_{}_{}",
        vendor.get_name(),
        major,
        OS,
        *ARCHITECTURE
    ))
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
            let mut options = fs::OpenOptions::new();
            options.read(true);
            #[cfg(windows)]
            options.share_mode(1); // FILE_SHARE_READ survives handle inheritance.
            let lease = match options.open(generation.join("in-use")).await {
                Ok(file) => {
                    let file = file.into_std().await;
                    if file.try_lock_shared().is_err() {
                        continue;
                    }
                    Some(file)
                }
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(_) => continue,
            };
            if let Ok(java) = runtime(&generation, lease).await {
                return Ok(java);
            }
        }
    }
    runtime(&root.join(format!("{}_{}", vendor.get_name(), major)), None).await
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

async fn prune(dir: &Path) -> Result<()> {
    let active = fs::read_to_string(dir.join("active")).await?;
    let mut entries = fs::read_dir(dir).await?;
    let mut pending = false;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        if name
            .to_str()
            .and_then(|s| s.strip_prefix(".obsolete-"))
            .is_some_and(valid_hash)
            && entry.file_type().await?.is_dir()
            && fs::try_exists(entry.path().join("in-use")).await?
        {
            fs::remove_dir_all(entry.path()).await?;
            continue;
        }
        let Some(hash) = name.to_str().filter(|name| valid_hash(name)) else {
            continue;
        };
        if active.lines().any(|kept| hash == kept) || !entry.file_type().await?.is_dir() {
            continue;
        }
        let lease = match fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(entry.path().join("in-use"))
            .await
        {
            Ok(file) => file.into_std().await,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(_) => {
                pending = true;
                continue;
            }
        };
        if lease.try_lock().is_err() {
            pending = true;
            continue;
        }
        // The rename removes it from readers' search path before releasing the lease.
        let trash = dir.join(format!(".obsolete-{hash}"));
        fs::rename(entry.path(), &trash).await?;
        drop(lease);
        fs::remove_dir_all(&trash).await?;
    }
    if !pending {
        fs::remove_file(dir.join("prune-pending")).await?;
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
    if fs::try_exists(&dest).await? {
        let existing = runtime(&dest, None).await?;
        ensure!(
            running_version(existing.path()).await? == version,
            "Cached package version mismatch"
        );
        fs::remove_dir_all(stage).await?;
    } else {
        fs::File::create_new(stage.join("in-use")).await?;
        fs::rename(stage, &dest).await?;
    }
    let previous = fs::read_to_string(dir.join("active"))
        .await
        .unwrap_or_default();
    let mut active = package.sha256.clone();
    for old in previous
        .lines()
        .filter(|hash| valid_hash(hash) && *hash != package.sha256)
    {
        if runtime(&dir.join(old), None).await.is_ok() {
            active.push('\n');
            active.push_str(old);
            break;
        }
    }
    fs::write(dir.join("active.tmp"), active).await?;
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
    use tokio::io::AsyncWriteExt;

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
    async fn selects_only_completed_upgrades() -> Result<()> {
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
        let hidden = next.with_extension("hidden");
        fs::rename(&next, &hidden).await?;
        assert_eq!(
            find_cached_java_binary(root.path(), vendor, &21)
                .await?
                .path(),
            first
        );
        fs::rename(hidden, &next).await?;
        let (third, bytes) = archive("21.0.10+1").await?;
        activate(&dir, 21, &third, Some(&old.version), &bytes).await?;
        assert!(fs::try_exists(&first).await?);
        game.kill().await?;
        prune(&dir).await?;
        assert!(!fs::try_exists(&first).await?);
        assert!(fs::try_exists(dir.join(new.sha256)).await?);
        Ok(())
    }
}
