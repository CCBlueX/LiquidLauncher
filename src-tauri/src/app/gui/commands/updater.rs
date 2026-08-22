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
use crate::{minecraft::progress::ProgressUpdate, LAUNCHER_VERSION};
use anyhow::Result;
use std::{
    env,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::oneshot::channel;
use tracing::{debug, debug_span, error, info, Instrument};

const SKIP_UPDATE_ENV: &str = "LIQUIDLAUNCHER_SKIP_UPDATE";
const REPORT_INTERVAL: Duration = Duration::from_millis(250);

/// Checks for a launcher update and installs it, then restarts.
#[tauri::command]
pub(crate) async fn check_for_updates(app: AppHandle) -> Result<(), String> {
    let span = debug_span!("update_check");

    match update(app.clone()).instrument(span.clone()).await {
        Ok(()) => Ok(()),
        Err(e) => {
            error!(parent: &span, "Update failed: {:?}", e);
            notify_failure(&app, &e).await;
            Err(format!("{}", e))
        }
    }
}

async fn notify_failure(app: &AppHandle, error: &anyhow::Error) {
    let (tx, rx) = channel();

    let mut dialog = app
        .dialog()
        .message(format!(
            "The launcher could not update itself and will try again on the next start.\n\n{}",
            error
        ))
        .title("Launcher update failed")
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::Ok);

    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }

    dialog.show(move |_| {
        let _ = tx.send(());
    });

    let _ = rx.await;
}

async fn update(app: AppHandle) -> Result<()> {
    if let Some(value) = env::var_os(SKIP_UPDATE_ENV) {
        info!(
            "Skipping update check, {} is set to {:?}",
            SKIP_UPDATE_ENV, value
        );
        return Ok(());
    }

    info!(
        "Checking for launcher updates (current v{})",
        LAUNCHER_VERSION
    );

    let Some(update) = app.updater()?.check().await? else {
        info!("Launcher is up to date (v{})", LAUNCHER_VERSION);
        return Ok(());
    };

    info!(
        "Update available: v{} -> v{} for {} (published {})",
        update.current_version,
        update.version,
        update.target,
        update
            .date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    if let Some(body) = &update.body {
        debug!("Release notes: {}", body);
    }

    info!(
        "Downloading update v{} from {}",
        update.version, update.download_url
    );
    report(
        &app,
        ProgressUpdate::set_label(format!("Updating launcher to v{}...", update.version)),
    );

    let mut downloaded = 0u64;
    let mut max_reported = false;
    let mut tick = Instant::now();
    let mut tick_bytes = 0u64;
    let mut logged_percent = 0u64;

    update
        .download_and_install(
            |chunk_length, content_length| {
                downloaded += chunk_length as u64;

                if let (false, Some(total)) = (max_reported, content_length) {
                    max_reported = true;
                    report(&app, ProgressUpdate::SetMax(total));
                }

                let elapsed = tick.elapsed();
                if elapsed < REPORT_INTERVAL {
                    return;
                }
                let speed = (downloaded - tick_bytes) * 1000 / elapsed.as_millis() as u64;
                tick = Instant::now();
                tick_bytes = downloaded;

                report(&app, ProgressUpdate::SetProgress(downloaded));
                report(&app, ProgressUpdate::SetDownloadSpeed(speed));

                if let Some(total) = content_length.filter(|total| *total > 0) {
                    let percent = downloaded * 100 / total;
                    if percent >= logged_percent + 10 {
                        logged_percent = percent;
                        debug!(
                            "Downloaded {}% of update ({} of {} bytes)",
                            percent, downloaded, total
                        );
                    }
                }
            },
            || {
                info!("Download finished, installing update...");
                report(&app, ProgressUpdate::set_label("Installing update..."));
                report(&app, ProgressUpdate::SetDownloadSpeed(0));
            },
        )
        .await?;

    info!("Update v{} installed, restarting launcher", update.version);
    app.restart()
}

fn report(app: &AppHandle, progress_update: ProgressUpdate) {
    if let Err(e) = app.emit("progress-update", &progress_update) {
        error!("Failed to report update progress: {:?}", e);
    }
}
