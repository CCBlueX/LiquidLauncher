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
 
// #![feature(exit_status_error)] - wait for feature to be stable
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{fs, io};
use once_cell::sync::Lazy;
use anyhow::Result;
use directories::ProjectDirs;
use reqwest::Client;
use tracing::{debug, info, error};
use tracing_subscriber::layer::SubscriberExt;
use utils::ARCHITECTURE;

use crate::utils::{OS, OS_VERSION};

pub mod app;
pub mod minecraft;

mod error;
mod utils;

const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
static LAUNCHER_DIRECTORY: Lazy<ProjectDirs> = Lazy::new(|| {
    match ProjectDirs::from("net", "CCBlueX",  "LiquidLauncher") {
        Some(proj_dirs) => proj_dirs,
        None => panic!("no application directory")
    }
});

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

/// HTTP Client with launcher agent
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    let client = reqwest::ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .build()
            .unwrap_or_else(|_| Client::new());
    
    client
});

/// Creates a directory tree if it doesn't exist
macro_rules! mkdir {
    ($path:expr) => {
        // Check if directory exists
        if !$path.exists() {
            // Create directory
            if let Err(e) = fs::create_dir_all($path) {
                error!("Failed to create directory {:?}: {}", $path, e);
            }
        }
    };
}

pub fn main() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};

    let log_folder = LAUNCHER_DIRECTORY.data_dir().join("logs");

    let file_appender = tracing_appender::rolling::hourly(log_folder, "launcher.log");

    let subscriber  = tracing_subscriber::registry()
        .with(EnvFilter::from("liquidlauncher=debug"))
        .with(
            fmt::Layer::new()
                .pretty()
                .with_ansi(true)
                .with_writer(io::stdout)
        )
        .with(
            fmt::Layer::new()
                .with_ansi(false)
                .with_writer(file_appender)
        );
    tracing::subscriber::set_global_default(subscriber).expect("Unable to set a global subscriber");

    info!("Starting LiquidLauncher v{}", LAUNCHER_VERSION);
    info!("OS: {:} {:} {:}", OS, ARCHITECTURE, OS_VERSION.to_string());

    // application directory
    info!("Creating application directory");
    debug!("Application directory: {:?}", LAUNCHER_DIRECTORY.data_dir());
    debug!("Config directory: {:?}", LAUNCHER_DIRECTORY.config_dir());
    mkdir!(LAUNCHER_DIRECTORY.data_dir());
    mkdir!(LAUNCHER_DIRECTORY.config_dir());

    // app
    info!("The GUI is starting...");
    app::gui::gui_main();

    info!("Launcher exited");
    Ok(())
}