use std::{path::{Path, PathBuf}, str::FromStr};
use futures::stream::{self, StreamExt};
use log::*;
use tokio::{fs, process::Command};
use crate::os::OS;
use path_absolutize::*;

use anyhow::Result;

use super::version::VersionProfile;
use std::fmt::Write;

const CONCURRENT_DOWNLOADS: usize = 10;

pub async fn launch(profile: VersionProfile) -> Result<()> {
    match profile {
        VersionProfile::V14(version_profile) => {
            unimplemented!();
        }
        VersionProfile::V21(version_profile) => {
            let mut class_path = String::new();

            // Client
            let versions_folder = Path::new("versions");

            // Check if json has client download (or doesn't require one)
            if let Some(client_download) = &version_profile.downloads.client {
                let client_folder = versions_folder.join(&version_profile.id);
                fs::create_dir_all(&client_folder).await?;

                let mut client_jar = client_folder.join(format!("{}.jar", &version_profile.id));

                // Add client jar to class path
                write!(class_path, "{}{}", &client_jar.absolutize().unwrap().to_str().unwrap(), OS.get_path_separator())?;

                // Download client jar
                if !client_jar.exists() {
                    client_download.download(&client_jar).await?;
                }
            }
            
            // Assets
            let assets_folder = Path::new("assets");
            let indexes_folder: PathBuf = assets_folder.join("indexes");
            let objects_folder: PathBuf = assets_folder.join("objects");
            fs::create_dir_all(&indexes_folder).await?;
            fs::create_dir_all(&objects_folder).await?;

            let asset_index = version_profile.asset_index_location.load_asset_index(&indexes_folder).await?;

            let _: Vec<Result<()>> = stream::iter(
                asset_index.objects.iter().map(|(_, asset_object)| asset_object.download(&objects_folder))
            ).buffer_unordered(CONCURRENT_DOWNLOADS).collect().await;

            // Libraries
            let libraries_folder = Path::new("libraries");
            let natives_folder = Path::new("natives");
            fs::create_dir_all(&natives_folder).await?;

            // todo: make library downloader compact and async
            
            for library in &version_profile.libraries {
                if let Some(artifact) = &library.downloads.artifact {
                    let library_path = libraries_folder.join(&artifact.path);

                    write!(class_path, "{}{}", &library_path.absolutize().unwrap().to_str().unwrap(), OS.get_path_separator())?;

                    if !library_path.exists() {
                        fs::create_dir_all(&library_path.parent().unwrap()).await?;
                        artifact.download(&library_path).await?;
                    }
                }

                if let Some(natives) = &library.natives {
                    if let Some(required_natives) = natives.get(&format!("{}", &OS)) {
                        debug!("required natives: {}", required_natives);

                        if let Some(classifiers) = &library.downloads.classifiers {
                            if let Some(artifact) = classifiers.get(required_natives) {
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
                        }else{
                            error!("missing classifiers, but natives required");
                            // where are the classifiers wtf
                        }
                    }
                }
            }

            // Game
            let mut command = Command::new("java");

            let game_dir = Path::new("gameDir");

            let mut command_arguments = Vec::new();

            // todo: cleanup and make compact

            for argument in &version_profile.arguments.jvm {
                if argument.rules.is_some() {
                    // todo: implement rules
                    continue;
                }

                match &argument.value {
                    super::version::ArgumentValue::SINGLE(value) => command_arguments.push(value.to_owned()),
                    super::version::ArgumentValue::VEC(vec) => command_arguments.append(&mut vec.clone())
                };
            }

            command_arguments.push(version_profile.main_class.to_owned());

            for argument in &version_profile.arguments.game {
                if argument.rules.is_some() {
                    // todo: implement rules
                    continue;
                }

                match &argument.value {
                    super::version::ArgumentValue::SINGLE(value) => command_arguments.push(value.to_owned()),
                    super::version::ArgumentValue::VEC(vec) => command_arguments.append(&mut vec.clone())
                };
            }

            let mapped: Vec<String> = command_arguments.iter().map(|s| s.replace("${auth_player_name}", "1zuna")
                .replace("${version_name}", "0.0.1")
                .replace("${game_directory}", &game_dir.absolutize().unwrap().to_str().unwrap())
                .replace("${assets_root}", &assets_folder.absolutize().unwrap().to_str().unwrap())
                .replace("${assets_index_name}", &version_profile.asset_index_location.id)
                .replace("${auth_uuid}", "2fc2c1dd-0234-48f6-94bb-4cb5812393ab")
                .replace("${auth_access_token}", "-")
                .replace("${user_type}", "legacy")
                .replace("${version_type}", &version_profile.version_type)
                .replace("${natives_directory}", &natives_folder.absolutize().unwrap().to_str().unwrap())
                .replace("${launcher_name}", "liquidlauncher")
                .replace("${launcher_version}", "1.0.0")
                .replace("${classpath}", &class_path[..class_path.len() - 1]))
                .collect();

            debug!("Arguments: {:?}", mapped);
            command.args(mapped);
            

            command.spawn()?
                .wait()
                .await?;
        }
    }
    Ok(())
}