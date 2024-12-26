use std::env;

use anyhow::{bail, Context, Result};
use tokio::fs;

const HOSTS_PATH: &str = "Windows\\System32\\drivers\\etc\\hosts";
const HOSTS: [&str; 4] = [
    "mojang.com",
    "minecraft.net",
    "liquidbounce.net",
    "ccbluex.net",
];

/// We have noticed many user have modified the hosts file to block the Minecraft authentication server.
/// This is likely by using a third-party program. Because LiquidLauncher requires access to the authentication server, we have to modify the hosts file to allow access.
/// we need to check the hosts file and alert the user if it has been modified.
pub async fn check_hosts_file() -> Result<()> {
    // Get location of Windows hosts file dynamically
    // "SystemDrive" env, if not assigned default to C:
    let system_drive = env::var("SystemDrive").unwrap_or("C:".to_string());
    let hosts_path = format!("{}\\{}", system_drive, HOSTS_PATH);

    // Check if hosts file exists, if not cancel this check with OK
    if let Ok(exists) = fs::try_exists(&hosts_path).await {
        if !exists {
            return Ok(());
        }
    }

    // Check if the hosts file has been modified
    let hosts_file = fs::read_to_string(&hosts_path)
        .await
        .context(format!("Failed to read hosts file at {}", hosts_path))?;

    let flagged_entries = hosts_file
        .lines()
        .filter(|line| {
            if line.starts_with('#') {
                return false;
            }

            let mut parts = line.split_whitespace();
            let _ = match parts.next() {
                Some(ip) => ip,
                None => return false,
            };
            let domain = match parts.next() {
                Some(domain) => domain,
                None => return false,
            };

            HOSTS.iter().any(|&entry| domain.contains(entry))
        })
        .collect::<Vec<_>>();

    if !flagged_entries.is_empty() {
        bail!(
            "The hosts file has been modified to block the Minecraft authentication server.\n\
            \n\
            Please remove the following entries from the hosts file:\n\
            {}\n\n\
            The file is located at:\n\
            {}",
            flagged_entries.join("\n"),
            hosts_path
        );
    }

    Ok(())
}
