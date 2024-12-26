use backon::{ExponentialBuilder, Retryable};
use tracing::{info, warn};
use crate::{utils, HTTP_CLIENT, LAUNCHER_VERSION};

#[tauri::command]
pub(crate) async fn get_launcher_version() -> Result<String, String> {
    Ok(LAUNCHER_VERSION.to_string())
}

#[tauri::command]
pub(crate) async fn check_health() -> Result<(), String> {
    #[cfg(windows)]
    {
        use crate::utils::check_hosts_file;
        info!("Checking hosts file...");
        check_hosts_file().await.map_err(|e| format!("{}", e))?;
    }

    info!("Checking online status");
    (|| async { HTTP_CLIENT
        .get("https://api.liquidbounce.net/")
        .send()
        .await
    })
        .retry(ExponentialBuilder::default())
        .notify(|err, dur| {
            warn!("Failed to check health. Retrying in {:?}. Error: {}", dur, err);
        })
        .await
        .map_err(|e| format!("unable to connect to api.liquidbounce.net: {:}", e))?
        .error_for_status()
        .map_err(|e| format!("api.liquidbounce.net returned an error: {:}", e))?;
    info!("Online status check successful");
    Ok(())
}

#[tauri::command]
pub(crate) fn sys_memory() -> u64 {
    utils::sys_memory() / (1024 * 1024)
}