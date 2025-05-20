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
use tauri::{Listener, Manager, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::time::sleep;
use tracing::{debug, info};

use super::gui::ShareableWindow;

const MAX_DOWNLOAD_ATTEMPTS: u8 = 2;

pub async fn open_download_page(
    url: &str,
    launcher_data: &LauncherData<ShareableWindow>,
) -> Result<String> {
    let download_page: Url = url.parse()
        .context("Failed to parse download page URL")?;

    let mut count = 0;

    let url = loop {
        count += 1;

        if count > MAX_DOWNLOAD_ATTEMPTS {
            bail!("Failed to open download page after {} attempts.\n\n\
            Please do not close the download window. Instead proceed with the download by pressing on 'Continue' and then 'Download'.\n\n\
            If the download window does not appear, please try restarting LiquidLauncher with administrator privileges.\n\
            If this does not help, please install LiquidBounce manually (https://liquidbounce.net/docs/get-started/manual-installation).\n\
            Or try our advice at https://liquidbounce.net/docs/tutorials/fixing-liquidlauncher.", MAX_DOWNLOAD_ATTEMPTS);
        }

        launcher_data.progress_update(ProgressUpdate::SetLabel(format!(
            "Opening download page... (Attempt {}/{})",
            count, MAX_DOWNLOAD_ATTEMPTS
        )));

        match show_webview(download_page.clone(), &launcher_data.data).await {
            Ok(pid) => break pid,
            Err(e) => {
                launcher_data.log(&format!("Failed to open download page: {:?}", e));
                sleep(Duration::from_millis(500)).await;
            }
        }
    };

    Ok(url)
}

async fn show_webview(url: Url, window: &Arc<Mutex<tauri::Window>>) -> Result<String> {
    let window = window
        .lock()
        .map_err(|_| anyhow!("Failed to lock window"))?;
    let app = window.app_handle();
    let main_window = window.get_webview_window("main")
        .ok_or_else(|| anyhow!("Failed to get window"))?;
    let len = app.webview_windows().len();

    let download_view = WebviewWindowBuilder::new(app, format!("download_view-{}", len), WebviewUrl::External(url))
        .title("Download of LiquidBounce JAR")
        .visible(true)
        .always_on_top(true)
        .maximized(true)
        .center()
        .parent(&main_window)?
        .build()?;
    drop(window);

    // Wait for the download to finish
    let pid_cell = Arc::new(Mutex::new(None));
    let close_request = Arc::new(AtomicBool::new(false));
    let cloned_close_request = close_request.clone();
    let cloned_cell = pid_cell.clone();

    download_view.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            close_request.store(true, Ordering::SeqCst);
        }
    });

    download_view.once("download", move |event| {
        debug!("Download Event received: {:?}", event);
        let payload = event.payload();

        #[derive(Deserialize)]
        struct DownloadPayload {
            pid: String
        }

        let payload = serde_json::from_str::<DownloadPayload>(payload).unwrap();

        info!("Received PID: {}", payload.pid);
        *cloned_cell.lock().unwrap() = Some(payload.pid);
    });

    let pid = loop {
        // sleep for 100ms
        sleep(Duration::from_millis(100)).await;

        // check if we got the download link
        if let Ok(pid) = pid_cell.lock() {
            if let Some(pid) = pid.clone() {
                break pid;
            }
        }

        if cloned_close_request.load(Ordering::SeqCst) {
            let _ = download_view.hide();
            bail!(
                "Download view was closed before the file PID was received. \
            Aborting download..."
            );
        }

        download_view
            .is_visible()
            .context("Download view was closed unexpected")?;
    };

    let _ = download_view.destroy();

    Ok(pid)
}
