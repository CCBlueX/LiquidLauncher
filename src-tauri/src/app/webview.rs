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
 
use std::{sync::{Arc, Mutex}, time::Duration};
use serde::Deserialize;
use anyhow::{Context, Result};
use tracing::{info, debug};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::time::sleep;
use crate::utils::download_file;

pub(crate) async fn download_client<F>(url: &str, on_progress: F, window: &Arc<Mutex<tauri::Window>>) -> Result<Vec<u8>> where F : Fn(u64, u64){
    let download_page = format!("{}&liquidlauncher=1", url);
    let download_view = WebviewWindowBuilder::new(
        window.lock().unwrap().app_handle(),
        "download_view",
        WebviewUrl::External(download_page.parse().unwrap())
    ).title("Download of LiquidBounce")
        .center()
        .focused(true)
        .maximized(true)
        .build().unwrap();

    // Show and maximize the download view
    download_view.show().unwrap();
    download_view.maximize().unwrap();

    // Wait for the download to finish
    let download_link_cell = Arc::new(Mutex::new(None));
    let cloned_cell = download_link_cell.clone();

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
        if let Ok(mg) = download_link_cell.lock() {
            if let Some(received) = mg.clone() {
                break received;
            }
        }

        // check if the view is still open, is_visible will throw error when the window is closed
        download_view.is_visible()
            .with_context(|| "The download view was closed unexpected, cancelling.")?;
    };

    download_view.close().unwrap();

    info!("Downloading LiquidBounce from {}", url);
    return download_file(&url, on_progress).await;
}