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
 
use std::{sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex}, time::Duration};
use serde::Deserialize;
use anyhow::{anyhow, bail, Context, Result};
use tracing::{info, debug};
use tauri::{Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tokio::time::sleep;
use crate::minecraft::progress::{ProgressReceiver, ProgressUpdate};

use super::gui::log;

const MAX_DOWNLOAD_ATTEMPTS: u8 = 5;

pub async fn open_download_page(url: &str, on_progress: &impl ProgressReceiver, window: &Arc<Mutex<tauri::Window>>) -> Result<String> {
    let download_page: Url = format!("{}&liquidlauncher=1", url).parse()
        .context("Failed to parse download page URL")?;

    let mut count = 0;

    let url = loop {
        count += 1;

        if count > MAX_DOWNLOAD_ATTEMPTS {
            bail!("Failed to open download page after {} attempts", MAX_DOWNLOAD_ATTEMPTS);
        }

        log(&window, &format!("Opening download page... (Attempt {}/{})", count, MAX_DOWNLOAD_ATTEMPTS));
        on_progress.progress_update(ProgressUpdate::SetLabel(format!("Opening download page... (Attempt {}/{})", count, MAX_DOWNLOAD_ATTEMPTS)));

        match show_webview(download_page.clone(), window).await {
            Ok(url) => break url,
            Err(e) => {
                log(&window, &format!("Failed to open download page: {}", e));
                sleep(Duration::from_millis(500)).await;
            }
        }
    };

    Ok(url)
}

async fn show_webview(url: Url, window: &Arc<Mutex<tauri::Window>>) -> Result<String> {
    let download_view = WebviewWindowBuilder::new(
        window.lock()
            .map_err(|_| anyhow!("Unable to lock window due to poisoned mutex"))?
            .app_handle(),
        "download_view",
        WebviewUrl::External(url)
    ).title("Download of LiquidBounce")
        .center()
        .focused(true)
        .maximized(true)
        .always_on_top(true)
        .build()
        .context("Failed to create download view")?;

    // Show and maximize the download view
    download_view.show()
        .context("Failed to show the download view")?;
    download_view.maximize()
        .context("Failed to maximize the download view")?;

    // Wait for the download to finish
    let download_link_cell = Arc::new(Mutex::new(None));
    let close_request = Arc::new(AtomicBool::new(false));
    let cloned_close_request = close_request.clone();
    let cloned_cell = download_link_cell.clone();
    
    download_view.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            close_request.store(true, Ordering::SeqCst);
        }
    });

    download_view.once("download", move |event| {
        debug!("Download Event received: {:?}", event);
        let payload = event.payload();

        #[derive(Deserialize)]
        struct DownloadPayload {
            url: String
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
            bail!("Download view was closed before the download link was received. \
            Aborting download...");
        }

        download_view.is_visible()
            .context("Download view was closed unexpected")?;
    };

    let _ = download_view.destroy();

    Ok(url)
}