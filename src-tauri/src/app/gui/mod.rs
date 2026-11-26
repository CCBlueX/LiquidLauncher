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

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use commands::*;
use serde::Serialize;
use serde_json::json;
use tauri::{Emitter, Window};
use tokio::sync::oneshot;

use crate::app::builds::KeptBuild;
use crate::minecraft::instance::Instance;

pub type ShareableWindow = Arc<Mutex<Window>>;

pub struct RunnerInstance {
    pub terminator: Option<oneshot::Sender<()>>,
    pub instance: Instance,
}

pub type RunnerInstances = Arc<Mutex<BTreeMap<u32, RunnerInstance>>>;

#[derive(Clone)]
pub struct Runner {
    pub id: u32,
    pub window: ShareableWindow,
    pub instances: RunnerInstances,
}

impl Runner {
    pub fn emit(&self, event: &str, payload: impl Serialize + Clone) -> Result<()> {
        self.window
            .lock()
            .map_err(|_| anyhow!("Window lock is poisoned"))?
            .emit(event, payload)?;
        Ok(())
    }

    pub fn update(&self, change: impl FnOnce(&mut Instance)) {
        if let Some(runner) = self.instances.lock().unwrap().get_mut(&self.id) {
            change(&mut runner.instance);
        }
        let _ = self.emit("client-running", json!({ "id": self.id }));
    }
}

pub struct AppState {
    pub runner_instances: RunnerInstances,
    /// The build that launches, as fetched last.
    pub build: KeptBuild,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            runner_instances: RunnerInstances::default(),
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
            list_instances,
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
