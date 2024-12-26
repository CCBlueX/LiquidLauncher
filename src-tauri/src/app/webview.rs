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

use crate::minecraft::{
    launcher::LauncherData,
    progress::{ProgressReceiver, ProgressUpdate},
};
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{Listener, Manager, Url, WebviewWindowBuilder};
use tokio::time::sleep;
use tracing::{debug, info};

use super::gui::ShareableWindow;

const MAX_DOWNLOAD_ATTEMPTS: u8 = 2;

pub async fn open_download_page(
    url: &str,
    launcher_data: &LauncherData<ShareableWindow>,
) -> Result<String> {
    let download_page: Url = format!("{}&liquidlauncher=1", url)
        .parse()
        .context("Failed to parse download page URL")?;

    let mut count = 0;

    let url = loop {
        count += 1;

        if count > MAX_DOWNLOAD_ATTEMPTS {
            bail!("Failed to open download page after {} attempts.\n\n\
            If the download window does not appear, please try restarting LiquidLauncher with administrator privileges.\n\
            If this does not help, please install LiquidBounce manually (https://liquidbounce.net/docs/Tutorials/Installation).\n\
            Or try our advice at https://liquidbounce.net/docs/Tutorials/Fixing%20LiquidLauncher.", MAX_DOWNLOAD_ATTEMPTS);
        }

        launcher_data.progress_update(ProgressUpdate::SetLabel(format!(
            "Opening download page... (Attempt {}/{})",
            count, MAX_DOWNLOAD_ATTEMPTS
        )));

        match show_webview(download_page.clone(), &launcher_data.data).await {
            Ok(url) => break url,
            Err(e) => {
                launcher_data.log(&format!("Failed to open download page: {:?}", e));
                sleep(Duration::from_millis(500)).await;
            }
        }
    };

    Ok(url)
}

async fn show_webview(url: Url, window: &Arc<Mutex<tauri::Window>>) -> Result<String> {
    // Find download_view window from the window manager
    let mut download_view = {
        let window = window
            .lock()
            .map_err(|_| anyhow!("Failed to lock window"))?;

        match window.get_webview_window("download_view") {
            Some(window) => Ok(window),
            None => {
                // todo: do not hardcode index
                let config = window
                    .config()
                    .app
                    .windows
                    .get(1)
                    .context("Unable to find window config")?;

                WebviewWindowBuilder::from_config(window.app_handle(), config)
                    .map_err(|e| anyhow!("Failed to build window: {:?}", e))?
                    .build()
            }
        }
    }?;

    // Redirect the download view to the download page
    download_view.navigate(url)?;

    // Show and maximize the download view
    download_view
        .show()
        .context("Failed to show the download view")?;
    download_view
        .maximize()
        .context("Failed to maximize the download view")?;

    // Wait for the download to finish
    let download_link_cell = Arc::new(Mutex::new(None));
    let close_request = Arc::new(AtomicBool::new(false));
    let cloned_close_request = close_request.clone();
    let cloned_cell = download_link_cell.clone();

    download_view.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            close_request.store(true, Ordering::SeqCst);
        }
    });

    download_view.once("download", move |event| {
        debug!("Download Event received: {:?}", event);
        let payload = event.payload();

        #[derive(Deserialize)]
        struct DownloadPayload {
            url: String,
        }

        let payload = serde_json::from_str::<DownloadPayload>(payload).unwrap();

        info!("Received download link: {}", payload.url);
        *cloned_cell.lock().unwrap() = Some(payload.url);
    });

    let url = loop {
        // sleep for 100ms
        sleep(Duration::from_millis(100)).await;

        // check if we got the download link
        if let Ok(link) = download_link_cell.lock() {
            if let Some(link) = link.clone() {
                break link;
            }
        }

        if cloned_close_request.load(Ordering::SeqCst) {
            let _ = download_view.hide();
            bail!(
                "Download view was closed before the download link was received. \
            Aborting download..."
            );
        }

        download_view
            .is_visible()
            .context("Download view was closed unexpected")?;
    };

    let _ = download_view.hide();

    Ok(url)
}
