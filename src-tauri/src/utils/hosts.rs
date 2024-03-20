use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use tauri_plugin_shell::ShellExt;

const HOSTS_PATH: &str = "C:\\Windows\\System32\\drivers\\etc\\hosts";

/// We have noticed many user have modified the hosts file to block the Minecraft authentication server.
/// This is likely by using a third-party program. Because LiquidLauncher requires access to the authentication server, we have to modify the hosts file to allow access.
/// we need to check the hosts file and alert the user if it has been modified.
pub async fn check_hosts_file(window: &Arc<Mutex<tauri::Window>>) -> Result<()> {
    // Check if the hosts file has been modified
    let hosts_file = tokio::fs::read_to_string(HOSTS_PATH).await?;

    let flagged_entries = hosts_file.lines()
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
            
            domain.contains("mojang.com") || domain.contains("minecraft.net")
        })
        .collect::<Vec<_>>();
    
    if !flagged_entries.is_empty() {
        // Open guide on how to remove the entries
        window.lock().unwrap()
            .shell().open("https://liquidbounce.net/docs/Tutorials/Fixing%20hosts%20file%20issues", None)?;

        bail!(
            "The hosts file has been modified to block the Minecraft authentication server.\n\
            \n\
            Please remove the following entries from the hosts file:\n\
            {}\n\n\
            The file is located at:\n\
            {}", 
            flagged_entries.join("\n"), 
            HOSTS_PATH
        );
    }

    Ok(())
}
