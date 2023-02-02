use crate::error::LauncherError;
use anyhow::Result;

pub fn get_maven_artifact_path(artifact_id: &String) -> Result<String> {
    let split = artifact_id.split(':').collect::<Vec<_>>();

    if split.len() != 3 {
        return Err(LauncherError::InvalidVersionProfile(format!("Invalid artifact name: {}", artifact_id)).into());
    }

    Ok(format!("{}/{name}/{ver}/{name}-{ver}.jar", split[0].replace('.', "/"), name = split[1], ver = split[2]))
}