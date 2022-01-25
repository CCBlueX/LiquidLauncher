#[cfg(feature = "gui")]
#[macro_use]
extern crate sciter;

pub mod minecraft;
pub mod cloud;

mod interface;
mod error;
mod utils;

const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let args = std::env::args();
    let mut real_args = args.skip(1);

    if let Some(build_id) = real_args.next() {
        #[cfg(feature = "cli")]
            {
                let u_build_id = build_id.parse::<u32>().expect("build id not valid");
                interface::cli::cli_main(u_build_id);
                return;
            }

        eprintln!("This build does not support CLI.");
        return;
    }

    #[cfg(feature = "gui")]
        {
            interface::gui::gui_main();
            return;
        }

    eprintln!("This build does not support GUI.");
    return;
}