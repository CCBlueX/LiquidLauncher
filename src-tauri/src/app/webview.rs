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

const MSA_LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// Opens a child webview on `window` pointed at `authorize_url` (Microsoft's
/// sign-in page) and waits for it to navigate to Microsoft's "you can close
/// this window" completion page, returning that final URL so the caller can
/// read `code`/`error` from its query string. No redirect capture is done
/// via a local HTTP server: the completion navigation is intercepted and
/// cancelled before it loads, so the window never shows a blank page.
pub async fn show_msa_login_webview(window: &ShareableWindow, authorize_url: Url) -> Result<Url> {
    let redirect_prefix = minecraft_auth::msa::MsaEnvironment::Live.native_client_url();
    let redirect_cell = Arc::new(Mutex::new(None));
    let cloned_cell = redirect_cell.clone();

    let close_request = Arc::new(AtomicBool::new(false));
    let cloned_close_request = close_request.clone();

    let login_view = {
        let window = window
            .lock()
            .map_err(|_| anyhow!("Failed to lock window"))?;
        let app = window.app_handle();
        let main_window = window.get_webview_window("main")
            .ok_or_else(|| anyhow!("Failed to get window"))?;
        let len = app.webview_windows().len();

        WebviewWindowBuilder::new(app, format!("msa_login-{}", len), WebviewUrl::External(authorize_url))
            .title("Sign in with Microsoft")
            .visible(true)
            .always_on_top(true)
            .inner_size(480.0, 650.0)
            .center()
            .parent(&main_window)?
            // Each sign-in attempt starts from a clean slate — no cookies or storage
            // carried over from a previous login (which would otherwise silently
            // reuse a stale or wrong Microsoft session instead of prompting again).
            .incognito(true)
            .on_navigation(move |url| {
                // The expected completion redirect, and a defensive catch-all for any
                // non-http(s) scheme Microsoft might redirect to instead (e.g. a native
                // broker handoff) — the webview can't follow those, so treat them the
                // same way: capture the URL for its query string and cancel navigation
                // before the webview engine chokes on the unsupported scheme.
                let is_completion_redirect = url.as_str().starts_with(&redirect_prefix)
                    || !matches!(url.scheme(), "https" | "http");
                if is_completion_redirect {
                    *cloned_cell.lock().unwrap() = Some(url.clone());
                    return false;
                }
                true
            })
            .build()?
    };

    login_view.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            close_request.store(true, Ordering::SeqCst);
        }
    });

    let start = std::time::Instant::now();
    let redirect_url = loop {
        sleep(Duration::from_millis(100)).await;

        if let Ok(mut guard) = redirect_cell.lock() {
            if let Some(url) = guard.take() {
                break url;
            }
        }

        if cloned_close_request.load(Ordering::SeqCst) {
            let _ = login_view.hide();
            bail!("Microsoft sign-in window was closed before completing sign-in.");
        }

        if login_view.is_visible().is_err() {
            bail!("Microsoft sign-in window was closed unexpectedly.");
        }

        if start.elapsed() > MSA_LOGIN_TIMEOUT {
            let _ = login_view.destroy();
            bail!("Microsoft sign-in timed out.");
        }
    };

    let _ = login_view.destroy();
    Ok(redirect_url)
}
