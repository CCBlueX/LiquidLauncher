use std::path::Path;

use anyhow::{Context, Result};
use async_zip::tokio::read::fs::ZipFileReader;
use serde::Deserialize;
use tokio::fs;

#[derive(Deserialize)]
struct GameVersion {
    world_version: u32,
}

/// A new game directory starts in `language`, Minecraft's code for the launcher language.
/// LiquidBounce's Auto language follows the game's; an existing `options.txt` stays untouched.
pub async fn setup_options(game_dir: &Path, client_jar: &Path, language: &str) -> Result<()> {
    let options = game_dir.join("options.txt");
    if fs::try_exists(&options).await? {
        return Ok(());
    }

    // Without `version` the game upgrades the file from the oldest format, which skips its first-start screen.
    let version = world_version(client_jar).await?;
    fs::write(&options, format!("version:{version}\nlang:{language}\n"))
        .await
        .context("Failed to write options.txt")
}

/// The data version in the client jar's `version.json`.
async fn world_version(client_jar: &Path) -> Result<u32> {
    let zip = ZipFileReader::new(client_jar).await?;
    let index = zip
        .file()
        .entries()
        .iter()
        .position(|entry| entry.filename().as_str().is_ok_and(|name| name == "version.json"))
        .context("The client JAR has no version.json")?;
    let mut json = String::new();
    zip.reader_with_entry(index)
        .await?
        .read_to_string_checked(&mut json)
        .await?;
    Ok(serde_json::from_str::<GameVersion>(&json)?.world_version)
}

#[cfg(test)]
mod tests {
    use async_zip::base::write::ZipFileWriter;
    use async_zip::{Compression, ZipEntryBuilder};
    use tempfile::tempdir;
    use tokio::fs::File;

    use super::*;

    #[tokio::test]
    async fn starts_a_new_game_in_the_launcher_language() {
        let dir = tempdir().unwrap();
        let client_jar = dir.path().join("26.3.jar");
        let mut jar = ZipFileWriter::with_tokio(File::create(&client_jar).await.unwrap());
        let entry = ZipEntryBuilder::new("version.json".to_owned().into(), Compression::Deflate);
        jar.write_entry_whole(entry, br#"{ "id": "26.3", "world_version": 5023 }"#)
            .await
            .unwrap();
        jar.close().await.unwrap();

        setup_options(dir.path(), &client_jar, "de_de").await.unwrap();
        let options = dir.path().join("options.txt");
        assert_eq!(fs::read_to_string(&options).await.unwrap(), "version:5023\nlang:de_de\n");

        setup_options(dir.path(), &client_jar, "zh_cn").await.unwrap();
        assert_eq!(fs::read_to_string(&options).await.unwrap(), "version:5023\nlang:de_de\n");
    }
}
