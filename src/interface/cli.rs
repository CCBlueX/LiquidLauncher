use std::collections::{BTreeMap, HashMap};
use std::io::{stdin, Write};

use anyhow::{anyhow, Result};
use env_logger::Env;
use log::*;
use uuid::Uuid;

use crate::cloud::{Build, LauncherApi, LaunchManifest};
use crate::minecraft::launcher::{LauncherData, LaunchingParameter};
use crate::minecraft::prelauncher;
use crate::minecraft::progress::ProgressUpdate;
use crate::minecraft::version::VersionManifest;

pub(crate) fn cli_main(build_id: u32) {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("Failed to open runtime");

    let builds = rt.block_on(LauncherApi::load_all_builds())
        .expect("Failed to download version manifest");

    let target_build = match builds.iter().find(|x| x.build_id == build_id) {
        Some(x) => x,
        None => {
            eprintln!("The requested version was not found.");
            eprintln!();
            eprintln!("Available versions:");

            builds
                .iter()
                .for_each(|x| eprintln!("Build ID: {} ({}): {} {}, minecraft: {}", x.build_id, x.commit_id, x.branch, x.mc_version, x.lb_version));
            return;
        }
    };

    let result = rt.block_on(async move {
        run(target_build).await
    });

    if let Err(e) = result {
        println!("ERROR: {}", e);
    }
}

async fn run(build: &Build) -> Result<()> {
    let (_, rx) = tokio::sync::oneshot::channel();

    prelauncher::launch(&build,
        LaunchingParameter {
            auth_player_name: "cliuser".to_string(),
            auth_uuid: "069a79f4-44e9-4726-a5be-fca90e38aaf5".to_string(),
            auth_access_token: "-".to_string(),
            auth_xuid: "0".to_string(),
            clientid: Uuid::new_v4().to_string(),
            user_type: "legacy".to_string()
        },
        LauncherData {
            on_stdout: handle_stdout,
            on_stderr: handle_stdout,
            on_progress: handle_progress,
            terminator: rx,
            data: Box::new(()),
        }).await?;

    Ok(())
}


fn handle_stdout(value: &(), data: &[u8]) -> anyhow::Result<()> {
    std::io::stdout().lock().write_all(data)?;
    std::io::stdout().lock().flush()?;

    Ok(())
}

fn handle_progress(value: &(), progress_update: ProgressUpdate) -> anyhow::Result<()> {
    Ok(())
}