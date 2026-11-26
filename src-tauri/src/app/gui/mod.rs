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

use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
use anyhow::Result;
use commands::*;
#[cfg(windows)]
use os_info::Version;
#[cfg(windows)]
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, Window};
#[cfg(target_os = "linux")]
use tauri::{WebviewWindow, WindowEvent};
#[cfg(target_os = "linux")]
use tracing::info;
#[cfg(windows)]
use tracing::warn;
#[cfg(target_os = "linux")]
use window_vibrancy::{apply_wayland_blur, RoundedRect};

use crate::app::builds::KeptBuild;

pub type ShareableWindow = Arc<Mutex<Window>>;

pub struct RunnerInstance {
    pub terminator: tokio::sync::oneshot::Sender<()>,
}

pub struct AppState {
    pub runner_instance: Arc<Mutex<Option<RunnerInstance>>>,
    /// The build that launches, as fetched last.
    pub build: KeptBuild,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            runner_instance: Arc::new(Mutex::new(None)),
            build: Mutex::new(None),
        }
    }
}

mod commands;

/// Runs the GUI and returns when the window is closed.
pub fn gui_main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_wayland_nvidia_quirk::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(AppState::new())
        .setup(|app| {
            let Some(window) = app.get_webview_window("main") else {
                return Ok(());
            };

            // Acrylic lags while the window is dragged before Windows 11 22H2.
            #[cfg(windows)]
            {
                let acrylic = matches!(
                    os_info::get().version(),
                    Version::Semantic(_, _, build) if *build >= 22621
                );
                let effect = if acrylic {
                    Effect::Acrylic
                } else {
                    Effect::Blur
                };
                if let Err(error) = window.set_effects(EffectsBuilder::new().effect(effect).build())
                {
                    warn!("Failed to blur the window: {error}");
                }
            }

            // Inside the rounded corners of `.window.rounded` in Window.svelte, again on every resize.
            #[cfg(target_os = "linux")]
            {
                let blur = |window: &WebviewWindow| -> Result<()> {
                    let size = window
                        .inner_size()?
                        .to_logical::<u32>(window.scale_factor()?);
                    let rect = RoundedRect {
                        width: size.width,
                        height: size.height,
                        radius: 8,
                    };
                    apply_wayland_blur(window, Some(rect))?;
                    Ok(())
                };
                match blur(&window) {
                    Ok(()) => {
                        let resized = window.clone();
                        window.on_window_event(move |event| {
                            if matches!(
                                event,
                                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
                            ) {
                                let _ = blur(&resized);
                            }
                        });
                    }
                    Err(error) => info!("No window blur: {error:#}"),
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            setup_client,
            check_system,
            sys_memory,
            get_options,
            store_options,
            request_build,
            request_build_page,
            get_java_distributions,
            request_mods,
            run_client,
            login_offline,
            login_microsoft_device_code,
            login_microsoft_webview,
            client_account,
            client_account_authenticate,
            client_account_logout,
            logout,
            refresh,
            fetch_blog_posts,
            fetch_changelog,
            clear_data,
            default_data_folder_path,
            terminate,
            get_launcher_version,
            get_custom_mods,
            install_custom_mod,
            delete_custom_mod,
            get_minecraft_installation,
            check_for_updates,
            get_marketplace_library,
            browse_marketplace,
            get_marketplace_item,
            install_marketplace_item,
            remove_marketplace_item,
            modrinth_search,
            modrinth_install
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
