use std::{path::{Path, PathBuf}, str::FromStr};
use std::collections::HashSet;
use std::fmt::Write;
use std::io::Write as OtherWrite;
use std::ops::Add;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Error, Result};
use async_zip::read::seek::ZipFileReader;
use futures::stream::{self, StreamExt};
use futures::TryFutureExt;
use log::*;
use path_absolutize::*;
use tokio::{fs, process::Command};
use tokio::{fs::File, io::AsyncReadExt};
use tokio::fs::OpenOptions;

use crate::{LAUNCHER_VERSION, utils::os::OS};
use crate::cloud::LaunchManifest;
use crate::error::LauncherError;
use crate::minecraft::progress::{get_max, get_progress, ProgressReceiver, ProgressUpdate, ProgressUpdateSteps};
use crate::minecraft::rule_interpreter;
use crate::minecraft::version::LibraryDownloadInfo;
use crate::utils::{download_file, zip_extract};

use super::version::VersionProfile;

pub struct LauncherData<D: Send + Sync> {
    pub(crate) on_stdout: fn(&D, &[u8]) -> Result<()>,
    pub(crate) on_stderr: fn(&D, &[u8]) -> Result<()>,
    pub(crate) on_progress: fn(&D, ProgressUpdate) -> Result<()>,
    pub(crate) data: Box<D>,
    pub(crate) terminator: tokio::sync::oneshot::Receiver<()>
}

impl<D: Send + Sync> ProgressReceiver for LauncherData<D> {
    fn progress_update(&self, progress_update: ProgressUpdate) {
        info!("{:?}", progress_update);
        let _ = (self.on_progress)(&self.data, progress_update);
    }
}

// Sorry if I burn your cpu and connection
const CONCURRENT_LIBRARY_DOWNLOADS: usize = 10;
const CONCURRENT_ASSET_DOWNLOADS: usize = 100;

pub async fn launch<D: Send + Sync>(manifest: LaunchManifest, version_profile: VersionProfile, launching_parameter: LaunchingParameter, launcher_data: LauncherData<D>) -> Result<()> {
    let launcher_data_arc = Arc::new(launcher_data);

    let features: HashSet<String> = HashSet::new();
    let os_info = os_info::get();

    info!("Determined OS to be {} {}", os_info.os_type(), os_info.version());

    // JRE download
    info!("Downloading JRE...");
    launcher_data_arc.progress_update(ProgressUpdate::set_label("Downloading JRE..."));

    let java_executable = crate::minecraft::jre_downloader::jre_download(manifest.build.jre_version, &os_info, |a, b| {
        launcher_data_arc.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadJRE, get_progress(0, a, b), get_max(1)));
    }).await?;

    // Launch class path for JRE
    let mut class_path = String::new();

    // Client
    let versions_folder = Path::new("versions");

    // Check if json has client download (or doesn't require one)
    if let Some(client_download) = version_profile.downloads.as_ref().and_then(|x| x.client.as_ref()) {
        let client_folder = versions_folder.join(&version_profile.id);
        fs::create_dir_all(&client_folder).await?;

        let client_jar = client_folder.join(format!("{}.jar", &version_profile.id));

        // Add client jar to class path
        write!(class_path, "{}{}", &client_jar.absolutize().unwrap().to_str().unwrap(), OS.get_path_separator())?;

        // Download client jar
        if !client_jar.exists() {
            info!("Downloading client jar...");
            launcher_data_arc.progress_update(ProgressUpdate::set_label("Downloading client..."));

            let retrieved_bytes = download_file(&client_download.url, |a, b| {
                launcher_data_arc.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadClientJar, get_progress(0, a, b), get_max(1)));
            }).await?;

            tokio::fs::write(&client_jar, retrieved_bytes).await?;
        }
    } else {
        return Err(LauncherError::InvalidVersionProfile("No client JAR downloads were specified.".to_string()).into());
    }

    // Libraries
    let libraries_folder = Path::new("libraries");
    let natives_folder = Path::new("natives");
    fs::create_dir_all(&natives_folder).await?;

    let libraries_to_download = version_profile.libraries.iter().map(|x| x.to_owned()).collect::<Vec<_>>();
    // let libraries_downloaded = Arc::new(AtomicU64::new(0));
    let libraries_max = libraries_to_download.len() as u64;

    launcher_data_arc.progress_update(ProgressUpdate::set_label("Checking libraries..."));
    launcher_data_arc.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadLibraries, 0, libraries_max));

    let class_paths: Vec<Result<Option<String>>> = stream::iter(
        libraries_to_download.into_iter().filter_map(|library| {
            // let download_count = libraries_downloaded.clone();
            let data_clone = launcher_data_arc.clone();
            let folder_clone = libraries_folder.to_path_buf();

            if !rule_interpreter::check_condition(&library.rules, &features, &os_info).unwrap_or(false) {
                return None;
            }

            Some(async move {
                if let Some(natives) = &library.natives {
                    if let Some(required_natives) = natives.get(OS.get_simple_name()) {
                        if let Some(classifiers) = library.downloads.as_ref().and_then(|x| x.classifiers.as_ref()) {
                            if let Some(artifact) = classifiers.get(required_natives).map(LibraryDownloadInfo::from) {
                                let path = artifact.download(library.name, folder_clone.as_path(), data_clone).await?;

                                info!("Natives zip extract: {:?}", path);
                                let file = OpenOptions::new().read(true).open(path).await?;
                                zip_extract(file, natives_folder).await?;
                            }
                        } else {
                            return Err(LauncherError::InvalidVersionProfile("missing classifiers, but natives required.".to_string()).into());
                        }
                    }

                    return Ok(None);
                }

                // Download regular artifact
                let artifact = library.get_library_download()?;
                let path = artifact.download(library.name, folder_clone.as_path(), data_clone).await?;

                // Natives are not included in the classpath
                return if library.natives.is_none() {
                    return Ok(path.absolutize().unwrap().to_str().map(|x| x.to_string()))
                } else {
                    Ok(None)
                };
            })
        })
    ).buffer_unordered(CONCURRENT_LIBRARY_DOWNLOADS).collect().await;
    for x in class_paths {
        if let Some(library_path) = x? {
            write!(class_path, "{}{}", &library_path, OS.get_path_separator())?;
        }
    }

    launcher_data_arc.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadLibraries, libraries_max, libraries_max));

    // Assets
    let assets_folder = Path::new("assets");
    let indexes_folder: PathBuf = assets_folder.join("indexes");
    let objects_folder: PathBuf = assets_folder.join("objects");

    fs::create_dir_all(&indexes_folder).await?;
    fs::create_dir_all(&objects_folder).await?;

    let asset_index_location = version_profile.asset_index_location.as_ref().ok_or_else(|| LauncherError::InvalidVersionProfile("Asset index unspecified".to_string()))?;
    let asset_index = asset_index_location.load_asset_index(&indexes_folder).await?;
    let asset_objects_to_download = asset_index.objects.values().map(|x| x.to_owned()).collect::<Vec<_>>();
    let assets_downloaded = Arc::new(AtomicU64::new(0));
    let asset_max = asset_objects_to_download.len() as u64;

    launcher_data_arc.progress_update(ProgressUpdate::set_label("Checking assets..."));
    launcher_data_arc.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadAssets, 0, asset_max));

    let _: Vec<Result<()>> = stream::iter(
        asset_objects_to_download.into_iter().map(|asset_object| {
            let download_count = assets_downloaded.clone();
            let data_clone = launcher_data_arc.clone();
            let folder_clone = objects_folder.clone();

            async move {
                let hash = asset_object.hash.clone();
                match asset_object.download_destructing(folder_clone, data_clone.clone()).await {
                    Ok(downloaded) => {
                        let curr = download_count.fetch_add(1, Ordering::Relaxed);

                        if downloaded {
                            // the progress bar is only being updated when a asset has been downloaded to improve speeds
                            data_clone.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadAssets, curr, asset_max));
                            data_clone.progress_update(ProgressUpdate::set_label(format!("Downloaded asset {}", hash)));
                        }
                    },
                    Err(err) => error!("Unable to download asset {}: {:?}", hash, err)
                }

                Ok(())
            }
        })
    ).buffer_unordered(CONCURRENT_ASSET_DOWNLOADS).collect().await;

    launcher_data_arc.progress_update(ProgressUpdate::set_for_step(ProgressUpdateSteps::DownloadAssets, asset_max, asset_max));

    // Game
    let mut command = Command::new(java_executable);

    let game_dir = Path::new("gameDir");

    let mut command_arguments = Vec::new();

    // todo: cleanup and make compact

    // JVM Args
    version_profile.arguments.add_jvm_args_to_vec(&mut command_arguments, &features, &os_info)?;

    // Main class
    command_arguments.push(version_profile.main_class.as_ref().ok_or_else(|| LauncherError::InvalidVersionProfile("Main class unspecified".to_string()))?.to_owned());

    // Game args
    version_profile.arguments.add_game_args_to_vec(&mut command_arguments, &features, &os_info)?;

    let mut mapped: Vec<String> = Vec::with_capacity(command_arguments.len());

    for x in command_arguments.iter() {
        mapped.push(
            process_templates(x, |output, param| {
                match param {
                    "auth_player_name" => output.push_str(&launching_parameter.auth_player_name),
                    "version_name" => output.push_str(&version_profile.id),
                    "game_directory" => output.push_str(&game_dir.absolutize().unwrap().to_str().unwrap()),
                    "assets_root" => output.push_str(&assets_folder.absolutize().unwrap().to_str().unwrap()),
                    "assets_index_name" => output.push_str(&asset_index_location.id),
                    "auth_uuid" => output.push_str(&launching_parameter.auth_uuid),
                    "auth_access_token" => output.push_str(&launching_parameter.auth_access_token),
                    "user_type" => output.push_str(&launching_parameter.user_type),
                    "version_type" => output.push_str(&version_profile.version_type),
                    "natives_directory" => output.push_str(&natives_folder.absolutize().unwrap().to_str().unwrap()),
                    "launcher_name" => output.push_str("LiquidLauncher"),
                    "launcher_version" => output.push_str(LAUNCHER_VERSION),
                    "classpath" => output.push_str(&class_path[..class_path.len() - 1]),
                    "user_properties" => output.push_str("{}"),
                    "clientid" => output.push_str(&launching_parameter.clientid),
                    "auth_xuid" => output.push_str(&launching_parameter.auth_xuid),
                    _ => return Err(LauncherError::UnknownTemplateParameter(param.to_owned()).into())
                };

                Ok(())
            })?
        );
    }


    launcher_data_arc.progress_update(ProgressUpdate::set_label("Launching..."));
    launcher_data_arc.progress_update(ProgressUpdate::set_to_max());

    debug!("MC-Arguments: {}", &mapped.join(" "));
    command.args(mapped);

    command
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());

    debug!("Launching with arguments: {:?}", &command);

    let mut running_task = command.spawn()?;

    let mut stdout = running_task.stdout.take().unwrap();
    let mut stderr = running_task.stderr.take().unwrap();

    let mut stdout_buf = vec![0; 1024];
    let mut stderr_buf = vec![0; 1024];

    let launcher_data = Arc::try_unwrap(launcher_data_arc).unwrap_or_else(|_| panic!());

    let terminator = launcher_data.terminator;

    tokio::pin!(terminator);

    loop {
        tokio::select! {
            read_len = stdout.read(&mut stdout_buf) => (launcher_data.on_stdout)(&launcher_data.data, &stdout_buf[..read_len?]).unwrap(),
            read_len = stderr.read(&mut stderr_buf) => (launcher_data.on_stderr)(&launcher_data.data, &stderr_buf[..read_len?]).unwrap(),
            _ = &mut terminator => {
                // todo: might cause issues with fabric error panel
                // running_task.kill().await?;

                break;
            },
            _ = running_task.wait() => {
                break;
            },
        }
    }

    Ok(())
}

pub struct LaunchingParameter {
    pub auth_player_name: String,
    pub auth_uuid: String,
    pub auth_access_token: String,
    pub auth_xuid: String,
    pub clientid: String,
    pub user_type: String
}

fn process_templates<F: Fn(&mut String, &str) -> Result<()>>(input: &String, retriever: F) -> Result<String> {
    let mut output = String::with_capacity(input.len() * 3 / 2);

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek().map_or(false, |&x| x == '{') {
            // Consuuuuume the '{'
            chars.next();

            let mut template_arg = String::with_capacity(input.len() - 3);

            let mut c;

            loop {
                c = chars.next().ok_or_else(|| LauncherError::InvalidVersionProfile("invalid template, missing '}'".to_string()))?;

                if c == '}' {
                    break;
                }
                if !matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9') {
                    return Err(LauncherError::InvalidVersionProfile(format!("invalid character in template: '{}'", c)).into());
                }

                template_arg.push(c);
            }

            retriever(&mut output, template_arg.as_str())?;
            continue;
        }

        output.push(c);
    }

    return Ok(output);
}