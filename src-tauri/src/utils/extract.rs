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

use anyhow::{Context, Result};
use async_compression::tokio::bufread::GzipDecoder;
use async_zip::read::seek::ZipFileReader;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, OpenOptions};
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, BufReader};

/// Extracts everything from the ZIP archive to the output directory
///
/// Taken from https://github.com/Majored/rs-async-zip/blob/main/examples/file_extraction.rs
pub async fn zip_extract<R>(archive: R, out_dir: &Path) -> Result<()>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    let mut reader = ZipFileReader::new(archive).await?;
    for index in 0..reader.file().entries().len() {
        let entry = &reader.file().entries().get(index).unwrap().entry();
        let file_name = entry.filename();

        let path = out_dir.join(sanitize_file_path(file_name));
        // If the filename of the entry ends with '/', it is treated as a directory.
        // This is implemented by previous versions of this crate and the Python Standard Library.
        // https://docs.rs/async_zip/0.0.8/src/async_zip/read/mod.rs.html#63-65
        // https://github.com/python/cpython/blob/820ef62833bd2d84a141adedd9a05998595d6b6d/Lib/zipfile.py#L528
        let entry_is_dir = file_name.ends_with('/');

        let mut entry_reader = reader.entry(index).await?;

        if entry_is_dir {
            // The directory may have been created if iteration is out of order.
            if !path.exists() {
                create_dir_all(&path).await?;
            }
        } else {
            // Creates parent directories. They may not exist if iteration is out of order
            // or the archive does not contain directory entries.
            let parent = path.parent().unwrap();
            if !parent.is_dir() {
                create_dir_all(parent).await?;
            }

            let mut writer = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)
                .await
                .context("Failed to create extracted file")?;
            io::copy(&mut entry_reader, &mut writer).await?;
        }
    }
    Ok(())
}

pub async fn tar_gz_extract<R>(archive: R, out_dir: &Path) -> Result<()>
where
    R: AsyncRead + AsyncSeek + Unpin,
{
    let mut decoder = GzipDecoder::new(BufReader::new(archive));
    let mut decoded_data: Vec<u8> = vec![];
    decoder.read_to_end(&mut decoded_data).await?;

    let mut ar = tokio_tar::Archive::new(&decoded_data[..]);
    ar.unpack(out_dir).await?;
    Ok(())
}

/// Returns a relative path without reserved names, redundant separators, ".", or "..".
fn sanitize_file_path(path: &str) -> PathBuf {
    // Replaces backwards slashes
    path.replace('\\', "/")
        // Sanitizes each component
        .split('/')
        .map(sanitize_filename::sanitize)
        .collect()
}
