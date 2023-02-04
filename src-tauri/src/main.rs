#![feature(once_cell)]
#![feature(exit_status_error)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{fs, io};
use once_cell::sync::Lazy;
use anyhow::Result;
use directories::ProjectDirs;
use tracing::{debug, Level};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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


    // application directory
    debug!("Creating launcher directories...");
    fs::create_dir_all(LAUNCHER_DIRECTORY.data_dir())?;
    fs::create_dir_all(LAUNCHER_DIRECTORY.config_dir())?;

    // app
    app::gui::gui_main();

    Ok(())
}