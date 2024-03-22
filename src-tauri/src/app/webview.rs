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
use anyhow::{bail, Context, Result};
use tracing::{info, debug};
use tauri::{Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tokio::time::sleep;
use crate::utils::download_file;

pub(crate) async fn download_client<F>(url: &str, on_progress: F, window: &Arc<Mutex<tauri::Window>>) -> Result<Vec<u8>> where F : Fn(u64, u64){
    let download_page: Url = format!("{}&liquidlauncher=1", url).parse()
        .context("Failed to parse download page URL")?;
    let download_view = WebviewWindowBuilder::new(
        window.lock().unwrap().app_handle(),
        "download_view",
        WebviewUrl::External(download_page)
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
        // sleep for 25ms
        sleep(Duration::from_millis(25)).await;

        // check if we got the download link
        if let Ok(mg) = download_link_cell.lock() {
            if let Some(received) = mg.clone() {
                break received;
            }
        }

        if cloned_close_request.load(Ordering::SeqCst) {
            bail!("Download view was closed before the download link was received. \
            Aborting download...");
        }
    };

    let _ = download_view.destroy();

    info!("Downloading LiquidBounce from {}", url);
    return download_file(&url, on_progress).await;
}