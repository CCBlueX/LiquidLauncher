#![windows_subsystem = "windows"]

#[cfg(feature = "gui")]
#[macro_use]
extern crate sciter;

use crate::app::option::LauncherOptions;

pub mod app;
pub mod minecraft;

mod error;
mod utils;

const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let args = std::env::args();
    let mut real_args = args.skip(1);

    let mut options = LauncherOptions::load().unwrap_or_default();
    if options.store().is_err() {
        println!("Failed to store options");
    }

    if let Some(build_id) = real_args.next() {
        #[cfg(feature = "cli")]
            {
                let u_build_id = build_id.parse::<u32>().expect("build id not valid");
                app::cli::cli_main(u_build_id);
                return;
            }

        eprintln!("This build does not support CLI.");
        return;
    }

    #[cfg(feature = "gui")]
        {
            app::gui::gui_main(options);
            return;
        }

    eprintln!("This build does not support GUI.");
    return;
}