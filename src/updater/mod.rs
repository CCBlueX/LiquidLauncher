use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Result};
use version_compare::Cmp;

/// This is currently the easiest way to check for updates,
/// but should be moved to api.liquidbounce.net to make sure we do not lose any control.
///
const GITHUB_RELEASE_API: &str = "https://api.github.com/repos/CCBlueX/LiquidLauncher/releases/latest";

/// The cargo version should always be in the format 0.1.0. No other characters included, plain version.
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

///
/// We compare the current version of the crate to the newest github release data, which we will pull from the GitHub API.
/// If the newest version number is higher than the current crate it is most likely outdated and requires an update on the user computers.
/// This allows us to notify the user and ask him for an automatic update.
///
pub async fn compare_versions() -> Result<(bool, GitHubReleaseData)> {
    let newest_data = version_data().await?;

    Ok((version_compare::compare_to(&newest_data.name, CURRENT_VERSION, Cmp::Gt).map_err(|_| anyhow!("unable to compare versions"))?, newest_data))
}

///
/// We pull the newest release version data from GitHub.
///
async fn version_data() -> Result<GitHubReleaseData> {
    let client = reqwest::ClientBuilder::new()
        .user_agent("LiquidLauncher")
        .build()?;

    Ok(client.get(GITHUB_RELEASE_API)
        .send().await?
        .json::<GitHubReleaseData>().await?)
}

/// Structured form of the GitHub API release data
#[derive(Serialize, Deserialize)]
pub struct GitHubReleaseData {
    url: String,
    id: i32,
    name: String,
    created_at: String,
    published_at: String
}