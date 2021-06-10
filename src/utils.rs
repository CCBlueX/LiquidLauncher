use crate::error::LauncherError;

pub(crate) fn get_maven_artifact_path(artifact_id: &String) -> anyhow::Result<String> {
    let split = artifact_id.split(":").collect::<Vec<_>>();

    if split.len() != 3 {
        return Err(LauncherError::InvalidVersionProfile(format!("Invalid artifact name: {}", artifact_id)).into());
    }

    Ok(format!("{}/{name}/{ver}/{name}-{ver}.jar", split[0].replace(".", "/"), name = split[1], ver = split[2]))
}

pub(crate) async fn download_file<F>(url: &str, on_progress: F) -> anyhow::Result<Vec<u8>> where F : Fn(u64, u64) {
    let mut response = reqwest::get(url).await?.error_for_status()?;

    let max_len = response.content_length().unwrap_or(0);

    let mut output = Vec::with_capacity(max_len as usize);

    let mut curr_len = 0;

    on_progress(0, max_len);

    while let Some(data) = response.chunk().await? {
        output.extend_from_slice(&data);

        curr_len += data.len();

        on_progress(curr_len as u64, max_len);
    }

    return Ok(output);
}