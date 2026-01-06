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

use commands::*;
use tauri::Window;

pub type ShareableWindow = Arc<Mutex<Window>>;

pub struct RunnerInstance {
    pub terminator: tokio::sync::oneshot::Sender<()>,
}

pub struct AppState {
    pub runner_instance: Arc<Mutex<Option<RunnerInstance>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            runner_instance: Arc::new(Mutex::new(None)),
        }
    }
}

mod commands;

/// Runs the GUI and returns when the window is closed.
pub fn gui_main() {
    tauri::Builder::default()
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
            request_branches,
            request_builds,
            request_mods,
            run_client,
            login_offline,
            login_microsoft,
            client_account_authenticate,
            client_account_update,
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
            modrinth_search,
            modrinth_get_version,
            modrinth_install,
            modrinth_update_mod,
            modrinth_check_updates,
            modrinth_get_installed,
            modrinth_sync_existing
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
