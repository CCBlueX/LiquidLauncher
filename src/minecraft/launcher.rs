use std::{path::{Path, PathBuf}, str::FromStr};
use futures::stream::{self, StreamExt};
use log::*;
use tokio::{fs, process::Command};
use crate::os::OS;
use path_absolutize::*;

use anyhow::{Result, bail, Error};

use super::version::VersionProfile;
use std::fmt::Write;
use std::collections::HashSet;
use crate::error::LauncherError;
use crate::minecraft::version::LibraryDownloadInfo;
use futures::TryFutureExt;
use std::ops::Add;

const CONCURRENT_DOWNLOADS: usize = 10;

pub async fn launch(version_profile: VersionProfile) -> Result<()> {
    let features: HashSet<String> = HashSet::new();
    let os_info = os_info::get();

    info!("Determined OS to be {} {}", os_info.os_type(), os_info.version());

    let mut class_path = String::new();

    // Client
    let versions_folder = Path::new("versions");

    // Check if json has client download (or doesn't require one)
    if let Some(client_download) = version_profile.downloads.as_ref().and_then(|x| x.client.as_ref()) {
        let client_folder = versions_folder.join(&version_profile.id);
        fs::create_dir_all(&client_folder).await?;

        let mut client_jar = client_folder.join(format!("{}.jar", &version_profile.id));

        // Add client jar to class path
        write!(class_path, "{}{}", &client_jar.absolutize().unwrap().to_str().unwrap(), OS.get_path_separator())?;

        // Download client jar
        if !client_jar.exists() {
            client_download.download(&client_jar).await?;
        }
    } else {
        return Err(LauncherError::InvalidVersionProfile("No client JAR downloads were specified.".to_string()).into());
    }

    // Assets
    let assets_folder = Path::new("assets");
    let indexes_folder: PathBuf = assets_folder.join("indexes");
    let objects_folder: PathBuf = assets_folder.join("objects");

    fs::create_dir_all(&indexes_folder).await?;
    fs::create_dir_all(&objects_folder).await?;

    let asset_index_location = version_profile.asset_index_location.as_ref().ok_or_else(|| LauncherError::InvalidVersionProfile("Asset index unspecified".to_string()))?;

    let asset_index = asset_index_location.load_asset_index(&indexes_folder).await?;

    let asset_objects_to_download = asset_index.objects.values().map(|x| x.to_owned()).collect::<Vec<_>>();

    let _: Vec<Result<()>> = stream::iter(
        asset_objects_to_download.into_iter().map(|asset_object| asset_object.download_destructing(&objects_folder))
    ).buffer_unordered(CONCURRENT_DOWNLOADS).collect().await;

    // Libraries
    let libraries_folder = Path::new("libraries");
    let natives_folder = Path::new("natives");
    fs::create_dir_all(&natives_folder).await?;

    // todo: make library downloader compact and async

    for library in &version_profile.libraries {
        if !crate::minecraft::rule_interpreter::check_condition(&library.rules, &features, &os_info)? {
            continue;
        }

        if let Some(natives) = &library.natives {
            if let Some(required_natives) = natives.get(&format!("{}", &OS)) {
                if let Some(classifiers) = library.downloads.as_ref().and_then(|x| x.classifiers.as_ref()) {
                    if let Some(artifact) = classifiers.get(required_natives).map(|x| LibraryDownloadInfo::from(x)) {
                        let library_path = libraries_folder.join(&artifact.path);

                        if !library_path.exists() {
                            fs::create_dir_all(&library_path.parent().unwrap()).await?;
                            artifact.download(&library_path).await?;
                        }

                        // todo: find async and safe alternative for zip extraction
                        // try https://github.com/zacps/zip-rs/tree/async-attempt2
                        let mut archive = zip::ZipArchive::new(std::fs::File::open(library_path).unwrap()).unwrap();

                        // todo: check for extract options in JSON
                        archive.extract(&natives_folder).unwrap();
                    }
                } else {
                    return Err(LauncherError::InvalidVersionProfile("missing classifiers, but natives required.".to_string()).into());
                }
            }

            continue;
        }

        let artifact = library.get_library_download()?;
        let library_path = libraries_folder.join(&artifact.path);

        if !library_path.exists() {
            fs::create_dir_all(&library_path.parent().unwrap()).await?;
            artifact.download(&library_path).await?;
        }

        // Natives are not included in the classpath
        if library.natives.is_none() {
            write!(class_path, "{}{}", &library_path.absolutize().unwrap().to_str().unwrap(), OS.get_path_separator())?;
        }
    }

    // Game
    let mut command = Command::new("java");

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
                    "auth_player_name" => output.push_str("1zuna"),
                    "version_name" => output.push_str("0.0.1"),
                    "game_directory" => output.push_str(&game_dir.absolutize().unwrap().to_str().unwrap()),
                    "assets_root" => output.push_str(&assets_folder.absolutize().unwrap().to_str().unwrap()),
                    "assets_index_name" => output.push_str(&asset_index_location.id),
                    "auth_uuid" => output.push_str("2fc2c1dd-0234-48f6-94bb-4cb5812393ab"),
                    "auth_access_token" => output.push_str("-"),
                    "user_type" => output.push_str("legacy"),
                    "version_type" => output.push_str(&version_profile.version_type),
                    "natives_directory" => output.push_str(&natives_folder.absolutize().unwrap().to_str().unwrap()),
                    "launcher_name" => output.push_str("liquidlauncher"),
                    "launcher_version" => output.push_str("1.0.0"),
                    "classpath" => output.push_str(&class_path[..class_path.len() - 1]),
                    "user_properties" => output.push_str("{}"),
                    _ => return Err(LauncherError::UnknownTemplateParameter(param.to_owned()).into())
                };

                Ok(())
            })?
        );
    }

    debug!("MC-Arguments: {:?}", &mapped);
    command.args(mapped);

    debug!("Launching with arguments: {:?}", &command);

    command.spawn()?
        .wait()
        .await?;

    Ok(())
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