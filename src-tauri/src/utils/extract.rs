use anyhow::{bail, Result};
use std::{
    env::current_dir,
    path::{Path, PathBuf},
};
use async_zip::read::seek::ZipFileReader;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io;

/// Extracts everything from the ZIP archive to the output directory
///
/// Taken from https://github.com/Majored/rs-async-zip/blob/main/examples/file_extraction.rs
pub async fn zip_extract(archive: File, out_dir: &Path) -> Result<()> {
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

            // Continue when file already exists
            if path.exists() {
                continue;
            }

            let mut writer = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
                .expect("Failed to create extracted file");
            io::copy(&mut entry_reader, &mut writer).await?;
        }
    }
    Ok(())
}

pub async fn tar_extract(archive: File, out_dir: &Path) -> Result<()> {

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