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

mod artifact;

use std::io;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use modloader::{Error, Jvm, Loaded, ModLoader};
use path_absolutize::Absolutize;
use tokio::fs;
use tokio::task::spawn_blocking;

use self::artifact::artifact;
use crate::app::client_api::{Build, Client, LaunchManifest, LoaderMod};
use crate::app::gui::Runner;
use crate::app::marketplace;
use crate::auth::ClientAccount;
use crate::minecraft::game_dir::GameDir;
use crate::minecraft::instance::{Instance, InstanceState};
use crate::minecraft::launcher::LauncherData;
use crate::minecraft::prelauncher;
use crate::minecraft::progress::{ProgressReceiver, ProgressUpdate};

const OLDEST: [u32; 2] = [26, 3];

impl Instance {
    pub fn all(root: &Path, build: Option<&Build>) -> Vec<Instance> {
        Self::among(&Jvm::all(), root, build)
    }

    fn among(jvms: &[Jvm], root: &Path, build: Option<&Build>) -> Vec<Instance> {
        let mut instances: Vec<_> = jvms
            .iter()
            .filter(|jvm| jvm.is_minecraft())
            .map(|jvm| {
                let directory = jvm.argument("--gameDir").map(Path::new);
                let profile = directory.or_else(|| {
                    jvm.command
                        .iter()
                        .find_map(|arg| arg.strip_prefix("-Djava.library.path="))
                        .map(Path::new)
                        .filter(|path| path.ends_with("natives"))
                });
                let profile = profile.and_then(|path| match path.file_name()?.to_str()? {
                    "minecraft" | ".minecraft" | "natives" => {
                        let parent = path.parent()?;
                        parent.parent()?.ends_with("instances").then_some(parent)
                    }
                    _ => Some(path),
                });
                let version = minecraft_version(jvm);

                Instance {
                    pid: Some(jvm.pid),
                    version: version.map(str::to_owned),
                    directory: directory.map(Path::to_path_buf),
                    profile: profile
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                        .map(str::to_owned),
                    username: jvm.argument("--username").map(str::to_owned),
                    state: if jvm.is_attachable() {
                        InstanceState::Running
                    } else {
                        InstanceState::Blocked
                    },
                    unsupported: version
                        .zip(build)
                        .and_then(|(version, build)| unsupported(version, build)),
                    ..Default::default()
                }
            })
            .filter(|instance| {
                let directory = instance.directory.as_deref();
                !directory.is_some_and(|directory| directory.starts_with(root))
            })
            .collect();
        instances.sort_by_key(|instance| instance.pid);
        instances
    }
}

fn minecraft_version(jvm: &Jvm) -> Option<&str> {
    if let Some(version) = jvm.argument("--fml.mcVersion") {
        return Some(version);
    }
    let classpath = jvm
        .argument("-cp")
        .or_else(|| jvm.argument("-classpath"))
        .or_else(|| jvm.argument("--class-path"));
    let game_jar = classpath.and_then(|classpath| {
        classpath
            .split(if cfg!(windows) { ';' } else { ':' })
            .find_map(|entry| {
                let path = Path::new(entry);
                let folder = path.parent()?;
                let layout = folder.parent()?;
                let jar = path.file_name()?.to_str()?.strip_suffix(".jar")?;
                if layout.ends_with("com/mojang/minecraft") {
                    jar.strip_prefix("minecraft-")?.strip_suffix("-client")
                } else if layout.ends_with("versions") && folder.ends_with(jar) {
                    Some(jar)
                } else {
                    None
                }
            })
    });
    let id = game_jar.or_else(|| jvm.argument("--version"))?;
    Some(
        id.strip_prefix("fabric-loader-")
            .and_then(|rest| rest.split_once('-'))
            .map_or(id, |(_, version)| version),
    )
}

fn unsupported(version: &str, build: &Build) -> Option<String> {
    let release: Vec<u32> = version
        .split('-')
        .next()?
        .split('.')
        .map(|part| part.parse().ok())
        .collect::<Option<_>>()?;

    if version != build.mc_version {
        Some(format!(
            "LiquidBounce {} is made for Minecraft {}",
            build.lb_version, build.mc_version
        ))
    } else if release.as_slice() < OLDEST.as_slice() {
        Some(format!(
            "LiquidBounce {} is too old to attach",
            build.lb_version
        ))
    } else {
        None
    }
}

pub(crate) async fn load(
    pid: u32,
    data: &Path,
    manifest: &LaunchManifest,
    additional_mods: &[LoaderMod],
    client: &Client,
    client_account: &Option<ClientAccount>,
    launcher_data: &LauncherData<Runner>,
) -> Result<(ModLoader, Loaded)> {
    let _speed_meter = launcher_data.start_speed_meter();

    launcher_data.progress_update(ProgressUpdate::set_max());
    launcher_data.progress_update(ProgressUpdate::SetProgress(0));

    let build = &manifest.build;
    let agent = artifact(data, manifest, "modloader", launcher_data).await?;
    let loader = ModLoader::new(pid, agent);

    launcher_data.progress_update(ProgressUpdate::set_label(
        "Asking the game for its version...",
    ));
    let instance = spawn_blocking({
        let loader = loader.clone();
        move || loader.instance()
    })
    .await?
    .context("Failed to reach the game")?;
    if !instance.is_client {
        bail!("LiquidBounce cannot be loaded into a server");
    }
    if instance.version != build.mc_version {
        bail!(
            "LiquidBounce {} is made for Minecraft {}, the game is Minecraft {}",
            build.lb_version,
            build.mc_version,
            instance.version
        );
    }

    prelauncher::setup_mods(
        data,
        manifest,
        additional_mods,
        client,
        client_account,
        launcher_data,
    )
    .await?;

    let game_dir = GameDir::new(data);
    let folder = game_dir.client();
    fs::create_dir_all(&folder).await?;
    let mut properties = vec![
        ("net.ccbluex.liquidbounce.api.url", client.url().to_owned()),
        (
            "net.ccbluex.liquidbounce.api.secure",
            client.is_secure().to_string(),
        ),
        (
            "net.ccbluex.liquidbounce.api.token",
            client.session_token().to_owned(),
        ),
    ];
    if loader.can_see(&folder) {
        let folder = folder
            .absolutize()?
            .to_str()
            .with_context(|| format!("{} is not UTF-8", folder.display()))?
            .to_owned();
        properties.push(("liquidbounce.folder", folder));
    } else {
        launcher_data.log(
            "The game runs in a sandbox without the launcher's files: \
             LiquidBounce keeps its settings in the game's own folder.",
        );
    }

    let library = artifact(data, manifest, "modloader-liquidbounce", launcher_data).await?;
    let Jars {
        optional,
        required,
        addons,
    } = grouped(jars_in(&game_dir.mods()).await?, &manifest.mods);

    let mut loaded = Loaded {
        mods: Vec::new(),
        present: Vec::new(),
        notes: Vec::new(),
    };
    let mut keep = |part: Loaded| {
        loaded.mods.extend(part.mods);
        loaded.present.extend(part.present);
        loaded.notes.extend(part.notes);
    };

    if !optional.is_empty() {
        let names: Vec<_> = optional
            .iter()
            .filter_map(|jar| jar.file_stem())
            .map(|name| name.to_string_lossy())
            .collect();
        launcher_data.log(&format!(
            "Not loaded into a running game, as most mods cannot be and some break it: {}. \
             A game LiquidLauncher starts has them, another game has those in its own mods folder.",
            names.join(", ")
        ));
    }

    launcher_data.progress_update(ProgressUpdate::set_label(format!(
        "Loading {} into the game...",
        build.commit_id
    )));
    let mut client_jars = vec![library];
    client_jars.extend(required);
    let with_addons = client_jars.iter().chain(&addons).cloned().collect();
    let part = match load_jars(&loader, with_addons, &properties).await {
        Err(Error::Refused(reason)) if !addons.is_empty() => {
            launcher_data.log(&format!("The add-ons were left out: {reason}"));
            load_jars(&loader, client_jars, &properties).await
        }
        result => result,
    };
    keep(part.context("Failed to load the client into the game")?);

    launcher_data.progress_update(ProgressUpdate::set_to_max());
    launcher_data.progress_update(ProgressUpdate::set_label(format!(
        "LiquidBounce runs in Minecraft {}",
        instance.version
    )));
    Ok((loader, loaded))
}

pub(crate) async fn unload(loader: &ModLoader) -> Result<Vec<String>, Error> {
    let loader = loader.clone();
    spawn_blocking(move || loader.unload("modloader-liquidbounce"))
        .await
        .map_err(io::Error::other)?
}

async fn load_jars(
    loader: &ModLoader,
    jars: Vec<PathBuf>,
    properties: &[(&'static str, String)],
) -> Result<Loaded, Error> {
    let (loader, properties) = (loader.clone(), properties.to_vec());
    spawn_blocking(move || {
        let properties: Vec<_> = properties
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        loader.load(&jars, &properties)
    })
    .await
    .map_err(io::Error::other)?
}

#[derive(Debug, Default, PartialEq)]
struct Jars {
    optional: Vec<PathBuf>,
    required: Vec<PathBuf>,
    addons: Vec<PathBuf>,
}

fn grouped(jars: Vec<PathBuf>, mods: &[LoaderMod]) -> Jars {
    let mut grouped = Jars::default();
    for jar in jars {
        let name = jar
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let is_required = |listed: &LoaderMod| {
            listed.required && name.strip_suffix(".jar") == Some(listed.name.as_str())
        };

        let group = if name.starts_with(marketplace::ADDON_PREFIX) {
            &mut grouped.addons
        } else if mods.iter().any(is_required) {
            &mut grouped.required
        } else {
            &mut grouped.optional
        };
        group.push(jar);
    }
    grouped
}

async fn jars_in(mods: &Path) -> Result<Vec<PathBuf>> {
    let mut jars = Vec::new();
    let mut entries = fs::read_dir(mods).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let is_jar = path.extension().is_some_and(|extension| extension == "jar");
        if is_jar && entry.file_type().await?.is_file() {
            jars.push(path);
        }
    }
    jars.sort();
    Ok(jars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::client_api::ModSource;
    use serde_json::{from_value, json};
    use tempfile::tempdir;

    fn build(mc_version: &str) -> Build {
        from_value(json!({
            "id": 1,
            "commitId": "adfddc0",
            "edition": "nextgen",
            "subsystem": "fabric",
            "lbVersion": "0.43.0",
            "mcVersion": mc_version,
            "release": true,
            "committedAt": "2026-11-20T13:24:01Z",
            "message": "",
            "downloadUrl": "",
            "jreVersion": 25,
        }))
        .unwrap()
    }

    #[test]
    fn lists_the_instances_of_other_launchers() {
        let jvm = |pid, command: &[&str]| Jvm {
            pid,
            command: command
                .iter()
                .map(|argument| argument.to_string())
                .collect(),
        };
        let root = Path::new("data").join("instances");
        let own = root.join("nextgen");
        let classpath = [
            "libs/net/fabricmc/fabric-loader/0.19.5/fabric-loader-0.19.5.jar",
            "libs/com/mojang/minecraft/1.21.11/minecraft-1.21.11-client.jar",
        ]
        .join(if cfg!(windows) { ";" } else { ":" });

        let instances = Instance::among(
            &[
                jvm(
                    30,
                    &[
                        "java",
                        "-Djava.library.path=instances/Survival with friends/natives",
                        "-cp",
                        &classpath,
                        "org.prismlauncher.EntryPoint",
                    ],
                ),
                jvm(20, &["java", "-jar", "gradle-wrapper.jar", "build"]),
                jvm(
                    10,
                    &[
                        "java",
                        "net.minecraft.client.main.Main",
                        "--username",
                        "Steve",
                        "--version",
                        "26.3",
                        "--gameDir",
                        "games",
                    ],
                ),
                jvm(
                    40,
                    &[
                        "java",
                        "net.minecraft.client.main.Main",
                        "--gameDir",
                        own.to_str().unwrap(),
                    ],
                ),
                jvm(
                    50,
                    &[
                        "java",
                        "-XX:+DisableAttachMechanism",
                        "net.minecraft.client.main.Main",
                    ],
                ),
            ],
            &root,
            Some(&build("26.3")),
        );

        assert_eq!(
            instances,
            [
                Instance {
                    pid: Some(10),
                    version: Some("26.3".to_owned()),
                    directory: Some(PathBuf::from("games")),
                    profile: Some("games".to_owned()),
                    username: Some("Steve".to_owned()),
                    ..Default::default()
                },
                Instance {
                    pid: Some(30),
                    version: Some("1.21.11".to_owned()),
                    profile: Some("Survival with friends".to_owned()),
                    unsupported: Some("LiquidBounce 0.43.0 is made for Minecraft 26.3".to_owned()),
                    ..Default::default()
                },
                Instance {
                    pid: Some(50),
                    state: InstanceState::Blocked,
                    ..Default::default()
                },
            ]
        );
    }

    #[test]
    fn shows_the_game_version_instead_of_the_launch_profile() {
        for (classpath, id, expected) in [
            ("versions/1.21.11/1.21.11.jar", "Survival", "1.21.11"),
            (
                "libs/fabric-api-1.2.jar",
                "fabric-loader-0.19.5-26.3",
                "26.3",
            ),
        ] {
            let jvm = Jvm {
                pid: 1,
                command: ["java", "-cp", classpath, "--version", id]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            };
            assert_eq!(minecraft_version(&jvm), Some(expected));
        }
    }

    #[test]
    fn tells_which_games_a_build_cannot_attach_to() {
        let made_for = |version| {
            Some(format!(
                "LiquidBounce 0.43.0 is made for Minecraft {version}"
            ))
        };
        let too_old = Some("LiquidBounce 0.43.0 is too old to attach".to_owned());

        assert_eq!(unsupported("26.3", &build("26.3")), None);
        assert_eq!(unsupported("26.10", &build("26.3")), made_for("26.3"));
        assert_eq!(unsupported("26.3", &build("1.21.11")), made_for("1.21.11"));
        assert_eq!(unsupported("1.21.11", &build("1.21.11")), too_old);
        assert_eq!(unsupported("26.4-pre-1", &build("26.3")), made_for("26.3"));
        assert_eq!(unsupported("26.3-rc-1", &build("26.3")), made_for("26.3"));
        assert_eq!(unsupported("26.4-pre-1", &build("26.4-pre-1")), None);
        assert_eq!(unsupported("1.21-pre1", &build("26.3")), made_for("26.3"));
        assert_eq!(unsupported("26w14a", &build("26.3")), None);
        assert_eq!(unsupported("Survival", &build("26.3")), None);
    }

    #[tokio::test]
    async fn hands_over_the_jar_files_in_name_order() -> Result<()> {
        let mods = tempdir()?;
        for name in ["b.jar", "notes.txt", "a.jar"] {
            fs::write(mods.path().join(name), b"").await?;
        }
        fs::create_dir(mods.path().join("folder.jar")).await?;

        let jars = jars_in(mods.path()).await?;

        assert_eq!(jars, [mods.path().join("a.jar"), mods.path().join("b.jar")]);
        Ok(())
    }

    #[test]
    fn groups_the_jars_by_how_they_load() {
        let listed = |name: &str, required| LoaderMod {
            required,
            enabled: true,
            name: name.to_owned(),
            source: ModSource::Local {
                file_name: String::new(),
            },
        };
        let jars = |names: &[&str]| -> Vec<PathBuf> {
            names
                .iter()
                .map(|name| Path::new("mods").join(name))
                .collect()
        };

        let grouped = grouped(
            jars(&[
                "Fabric API.jar",
                "LiquidBounce.jar",
                "Sodium.jar",
                "liquidbounce-addon-12-34.jar",
                "own-mod.jar",
            ]),
            &[
                listed("LiquidBounce", true),
                listed("Fabric API", true),
                listed("Sodium", false),
            ],
        );

        assert_eq!(
            grouped,
            Jars {
                optional: jars(&["Sodium.jar", "own-mod.jar"]),
                required: jars(&["Fabric API.jar", "LiquidBounce.jar"]),
                addons: jars(&["liquidbounce-addon-12-34.jar"]),
            }
        );
    }
}
