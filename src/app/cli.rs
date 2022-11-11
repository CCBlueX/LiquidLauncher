use std::io::{Write};

use anyhow::Result;
use directories::ProjectDirs;
use env_logger::Env;
use log::*;
use uuid::Uuid;

use crate::app::api::{Build, LauncherApi};
use crate::minecraft::launcher::{LauncherData, LaunchingParameter};
use crate::minecraft::prelauncher;
use crate::minecraft::progress::ProgressUpdate;
use rand::distributions::{Alphanumeric, DistString};

///
/// CLI of LiquidLauncher.
///
/// TODO: rework usage design and add missing options
///
pub fn cli_main(build_id: u32) {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("Failed to open runtime");

    let builds = rt.block_on(LauncherApi::load_all_builds())
        .expect("Failed to download version manifest");

    let random_username = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
    let parameters = LaunchingParameter {
        auth_player_name: random_username,
        auth_uuid: Uuid::new_v4().to_string(),
        auth_access_token: "-".to_string(),
        auth_xuid: "0".to_string(),
        clientid: Uuid::new_v4().to_string(),
        user_type: "legacy".to_string(),
        keep_launcher_open: true
    };

    let target_build = match builds.iter().find(|x| x.build_id == build_id) {
        Some(x) => x,
        None => {
            error!("The requested version was not found.");

            info!("Available versions:");

            builds
                .iter()
                .for_each(|x| info!("Build ID: {} ({}): {} {}, minecraft: {}", x.build_id, x.commit_id, x.branch, x.mc_version, x.lb_version));
            return;
        }
    };

    let result = rt.block_on(async move {
        run(parameters, target_build).await
    });

    if let Err(e) = result {
        error!("ERROR: {}", e);
    }
}

async fn run(parameters: LaunchingParameter, build: &Build) -> Result<()> {
    let (_, rx) = tokio::sync::oneshot::channel();

    prelauncher::launch(
        build,
        parameters,
        LauncherData {
            on_stdout: handle_stdout,
            on_stderr: handle_stdout,
            on_progress: handle_progress,
            terminator: rx,
            data: Box::new(()),
        }).await?;

    Ok(())
}


fn handle_stdout(value: &(), data: &[u8]) -> Result<()> {
    std::io::stdout().lock().write_all(data)?;
    std::io::stdout().lock().flush()?;

    Ok(())
}

fn handle_progress(value: &(), progress_update: ProgressUpdate) -> Result<()> {
    Ok(())
}