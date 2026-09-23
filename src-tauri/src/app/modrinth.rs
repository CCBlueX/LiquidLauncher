/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2026 CCBlueX
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

//! Mods from Modrinth. They load through its Maven repository like the recommended mods, so an
//! installed mod is a project and a version, and updating it changes the version.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use futures::future::join_all;
use reqwest::{IntoUrl, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;
use tracing::warn;

use crate::app::client_api::{LoaderMod, ModSource};
use crate::utils::sha1sum;
use crate::HTTP_CLIENT;

const API: &str = "https://api.modrinth.com/v2";
/// The name launch manifests give Modrinth's Maven repository.
const REPOSITORY: &str = "modrinth";
const GROUP: &str = "maven.modrinth";
const LISTED: u32 = 20;

/// A mod installed from Modrinth, as the options keep it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthMod {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub version: String,
    /// Resolves the jar. Version numbers repeat across loaders and Minecraft versions.
    pub version_id: String,
}

impl ModrinthMod {
    fn loader_mod(&self, enabled: bool) -> LoaderMod {
        LoaderMod {
            required: false,
            enabled,
            name: self.slug.clone(),
            source: ModSource::Repository {
                repository: REPOSITORY.to_owned(),
                artifact: format!("{GROUP}:{}:{}", self.project_id, self.version_id),
            },
        }
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
pub struct Hit {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    author: String,
    downloads: u64,
}

#[derive(Deserialize, Clone)]
struct Version {
    id: String,
    project_id: String,
    version_number: String,
}

#[derive(Deserialize)]
struct Project {
    id: String,
    slug: String,
    title: String,
}

async fn get<T: DeserializeOwned>(url: impl IntoUrl) -> Result<T> {
    Ok(HTTP_CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn endpoint(path: &str, query: &[(&str, &str)]) -> Result<Url> {
    Ok(Url::parse_with_params(&format!("{API}/{path}"), query)?)
}

/// Mods for the game, the most downloaded first without a query.
pub async fn search(query: Option<&str>, minecraft: &str, loader: &str) -> Result<Vec<Hit>> {
    let facets = json!([
        [format!("versions:{minecraft}")],
        [format!("categories:{loader}")],
        ["project_type:mod"]
    ])
    .to_string();
    let index = if query.is_some() {
        "relevance"
    } else {
        "downloads"
    };
    let limit = LISTED.to_string();
    let url = endpoint(
        "search",
        &[
            ("query", query.unwrap_or_default()),
            ("index", index),
            ("facets", &facets),
            ("limit", &limit),
        ],
    )?;

    let response: SearchResponse = get(url).await?;
    Ok(response.hits)
}

async fn newest_version(
    project_id: &str,
    minecraft: &str,
    loader: &str,
) -> Result<Option<Version>> {
    let url = endpoint(
        &format!("project/{project_id}/version"),
        &[
            ("loaders", &json!([loader]).to_string()),
            ("game_versions", &json!([minecraft]).to_string()),
        ],
    )?;
    let versions: Vec<Version> = get(url).await?;
    Ok(versions.into_iter().next())
}

/// The newest version of a project for the game, as it is installed.
pub async fn resolve(project_id: &str, minecraft: &str, loader: &str) -> Result<ModrinthMod> {
    let (project, version) = tokio::try_join!(
        get::<Project>(format!("{API}/project/{project_id}")),
        newest_version(project_id, minecraft, loader),
    )?;
    let version = version.with_context(|| {
        format!(
            "{} has no version for {} {minecraft}",
            project.title,
            loader_name(loader)
        )
    })?;

    Ok(ModrinthMod {
        project_id: project.id,
        slug: project.slug,
        title: project.title,
        version: version.version_number,
        version_id: version.id,
    })
}

fn loader_name(loader: &str) -> String {
    let mut chars = loader.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// What the game already holds, to tell the search hits apart.
pub struct Held<'a> {
    /// The mods of the build's launch manifest.
    pub launched: &'a [LoaderMod],
    /// The recommended mods, turned on or off as the options have them.
    pub recommended: &'a [LoaderMod],
    /// The projects installed from Modrinth or from files Modrinth knows.
    pub installed: &'a HashSet<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HitState {
    /// The build launches it.
    Included,
    Installed,
    /// A recommended mod that is turned off. Installing turns it on.
    Recommended {
        name: String,
    },
    Available,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    project_id: String,
    title: String,
    description: String,
    author: String,
    downloads: u64,
    state: HitState,
}

pub fn tell(hits: Vec<Hit>, held: &Held) -> Vec<SearchHit> {
    hits.into_iter()
        .map(|hit| SearchHit {
            state: state(&hit, held),
            project_id: hit.project_id,
            title: hit.title,
            description: hit.description,
            author: hit.author,
            downloads: hit.downloads,
        })
        .collect()
}

/// The group and name of a Maven artifact.
fn artifact(loader_mod: &LoaderMod) -> Option<(&str, &str)> {
    let ModSource::Repository { artifact, .. } = &loader_mod.source else {
        return None;
    };
    let mut parts = artifact.split(':');
    Some((parts.next()?, parts.next()?))
}

fn state(hit: &Hit, held: &Held) -> HitState {
    // Modrinth's Maven takes a project by id or slug; other repositories name Fabric Kotlin like
    // its slug does.
    let is_hit = |name: &str| name == hit.project_id || name.eq_ignore_ascii_case(&hit.slug);

    if held
        .launched
        .iter()
        .filter_map(artifact)
        .any(|(_, name)| is_hit(name))
    {
        return HitState::Included;
    }
    if held.installed.contains(&hit.project_id) {
        return HitState::Installed;
    }

    let recommended = held.recommended.iter().find(|recommended| {
        artifact(recommended).is_some_and(|(group, name)| group == GROUP && is_hit(name))
    });
    match recommended {
        Some(recommended) if recommended.required || recommended.enabled => HitState::Installed,
        Some(recommended) => HitState::Recommended {
            name: recommended.name.clone(),
        },
        None => HitState::Available,
    }
}

/// A mod added to the game, from a file or from Modrinth. Launching takes it as a [LoaderMod].
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomMod {
    #[serde(flatten)]
    loader_mod: LoaderMod,
    title: String,
    /// For a mod from Modrinth, and a file Modrinth knows.
    modrinth: Option<Tracked>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tracked {
    project_id: String,
    version: String,
    /// The newer version Modrinth has, once it was asked.
    update: Option<String>,
}

fn newer(version_id: &str, newest: Option<Version>) -> Option<String> {
    newest
        .filter(|newest| newest.id != version_id)
        .map(|newest| newest.version_number)
}

/// The mods installed from Modrinth, then the files in `dir`. `states` turns them on and off by
/// name. With `check`, Modrinth tells what is newer and which of the files it knows.
pub async fn custom_mods(
    dir: &Path,
    installed: &[ModrinthMod],
    states: &HashMap<String, bool>,
    minecraft: &str,
    loader: &str,
    check: bool,
) -> Result<Vec<CustomMod>> {
    let enabled = |name: &str| states.get(name).copied().unwrap_or(true);

    let mut mods: Vec<_> = installed
        .iter()
        .map(|installed| CustomMod {
            loader_mod: installed.loader_mod(enabled(&installed.slug)),
            title: installed.title.clone(),
            modrinth: Some(Tracked {
                project_id: installed.project_id.clone(),
                version: installed.version.clone(),
                update: None,
            }),
        })
        .collect();

    let files = jars(dir).await?;
    mods.extend(files.iter().map(|file_name| {
        let name = file_name.trim_end_matches(".jar").to_owned();
        CustomMod {
            loader_mod: LoaderMod {
                required: false,
                enabled: enabled(&name),
                name: name.clone(),
                source: ModSource::Local {
                    file_name: file_name.clone(),
                },
            },
            title: name,
            modrinth: None,
        }
    }));

    if check {
        let updates = join_all(installed.iter().map(|installed| async {
            newest_version(&installed.project_id, minecraft, loader)
                .await
                .map(|newest| newer(&installed.version_id, newest))
                .inspect_err(|error| warn!("Failed to check {}: {:?}", installed.title, error))
                .ok()
                .flatten()
        }));
        let (updates, known) = tokio::join!(updates, identify(dir, &files, minecraft, loader));

        for (row, update) in mods.iter_mut().zip(updates) {
            if let Some(tracked) = row.modrinth.as_mut() {
                tracked.update = update;
            }
        }

        match known {
            Ok(known) => {
                let files = &mut mods[installed.len()..];
                for (row, known) in files.iter_mut().zip(known) {
                    if let Some((title, tracked)) = known {
                        row.title = title;
                        row.modrinth = Some(tracked);
                    }
                }
            }
            Err(error) => warn!("Failed to look up mod files on Modrinth: {:?}", error),
        }
    }

    Ok(mods)
}

/// The jars in `dir`, by name.
async fn jars(dir: &Path) -> Result<Vec<String>> {
    let mut jars = vec![];
    let mut entries = match fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(jars),
        Err(error) => return Err(error.into()),
    };

    while let Some(entry) = entries.next_entry().await? {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().await?.is_file() && file_name.ends_with(".jar") {
            jars.push(file_name);
        }
    }
    jars.sort_by_key(|file_name| file_name.to_lowercase());
    Ok(jars)
}

/// The Modrinth version of each of `files`, where Modrinth knows the file.
async fn versions(dir: &Path, files: &[String]) -> Result<Vec<Option<Version>>> {
    if files.is_empty() {
        return Ok(vec![]);
    }

    let paths: Vec<PathBuf> = files.iter().map(|file_name| dir.join(file_name)).collect();
    let hashes = tokio::task::spawn_blocking(move || {
        paths
            .iter()
            .map(|path| sha1sum(path).ok())
            .collect::<Vec<_>>()
    })
    .await?;

    let known: HashMap<String, Version> = HTTP_CLIENT
        .post(format!("{API}/version_files"))
        .json(
            &json!({ "hashes": hashes.iter().flatten().collect::<Vec<_>>(), "algorithm": "sha1" }),
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(hashes
        .into_iter()
        .map(|hash| known.get(&hash?).cloned())
        .collect())
}

/// The projects of the files in `dir` that Modrinth knows.
pub async fn file_projects(dir: &Path) -> Result<HashSet<String>> {
    let files = jars(dir).await?;
    let versions = versions(dir, &files).await?;
    Ok(versions
        .into_iter()
        .flatten()
        .map(|version| version.project_id)
        .collect())
}

/// What Modrinth knows about each of `files`: the project's title, the version and what is newer.
async fn identify(
    dir: &Path,
    files: &[String],
    minecraft: &str,
    loader: &str,
) -> Result<Vec<Option<(String, Tracked)>>> {
    let versions = versions(dir, files).await?;
    let ids: Vec<_> = versions
        .iter()
        .flatten()
        .map(|version| version.project_id.as_str())
        .collect();
    if ids.is_empty() {
        return Ok(versions.iter().map(|_| None).collect());
    }

    let projects: Vec<Project> =
        get(endpoint("projects", &[("ids", &json!(ids).to_string())])?).await?;
    let projects: HashMap<_, _> = projects
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();

    let identified = versions.iter().map(|version| async {
        let version = version.as_ref()?;
        let project = projects.get(&version.project_id)?;
        let update = newest_version(&project.id, minecraft, loader)
            .await
            .map(|newest| newer(&version.id, newest))
            .inspect_err(|error| warn!("Failed to check {}: {:?}", project.title, error))
            .ok()
            .flatten();
        Some((
            project.title.clone(),
            Tracked {
                project_id: project.id.clone(),
                version: version.version_number.clone(),
                update,
            },
        ))
    });
    Ok(join_all(identified).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loader_mod(name: &str, artifact: &str, enabled: bool) -> LoaderMod {
        LoaderMod {
            required: false,
            enabled,
            name: name.to_owned(),
            source: ModSource::Repository {
                repository: "modrinth".to_owned(),
                artifact: artifact.to_owned(),
            },
        }
    }

    fn hit(project_id: &str, slug: &str) -> Hit {
        Hit {
            project_id: project_id.to_owned(),
            slug: slug.to_owned(),
            title: slug.to_owned(),
            description: String::new(),
            author: String::new(),
            downloads: 0,
        }
    }

    #[test]
    fn tells_what_the_game_holds() {
        // Build 16941 launches these, the recommended mods for 26.2 name their projects so.
        let launched = [
            loader_mod("Fabric API", "maven.modrinth:fabric-api:0.153.0+26.2", true),
            loader_mod(
                "Fabric Kotlin",
                "net.fabricmc:fabric-language-kotlin:1.13.12+kotlin.2.4.0",
                true,
            ),
        ];
        let recommended = [
            loader_mod("Sodium", "maven.modrinth:sodium:mc26.2-0.9.1-fabric", true),
            loader_mod("Iris", "maven.modrinth:iris:1.11.2+26.2-fabric", false),
            loader_mod(
                "ImmediatelyFast",
                "maven.modrinth:ImmediatelyFast:1.16.1+26.2-fabric",
                true,
            ),
            loader_mod(
                "Baritone",
                "baritone:baritone-fabric:26.2-1.11.1-18-g722bbec8",
                false,
            ),
        ];
        // Entity Culling, installed from Modrinth.
        let installed = HashSet::from(["NNAgCjsB".to_owned()]);
        let held = Held {
            launched: &launched,
            recommended: &recommended,
            installed: &installed,
        };

        let states: Vec<_> = [
            hit("P7dR8mSH", "fabric-api"),
            hit("Ha28R6CL", "fabric-language-kotlin"),
            hit("AANobbMI", "sodium"),
            hit("5ZwdcRci", "immediatelyfast"),
            hit("YL57xq9U", "iris"),
            hit("NNAgCjsB", "entityculling"),
            hit("fQEb0iXm", "krypton"),
            hit("baritone", "baritone"),
        ]
        .iter()
        .map(|hit| state(hit, &held))
        .collect();
        assert_eq!(
            states,
            [
                HitState::Included,
                HitState::Included,
                HitState::Installed,
                HitState::Installed,
                HitState::Recommended {
                    name: "Iris".to_owned()
                },
                HitState::Installed,
                HitState::Available,
                HitState::Available,
            ]
        );
    }

    #[test]
    fn loads_through_the_maven_repository() {
        let entity_culling = ModrinthMod {
            project_id: "NNAgCjsB".to_owned(),
            slug: "entityculling".to_owned(),
            title: "Entity Culling".to_owned(),
            version: "1.11.1".to_owned(),
            version_id: "5V6jfXWM".to_owned(),
        };
        let loader_mod = entity_culling.loader_mod(true);
        assert_eq!(loader_mod.name, "entityculling");
        assert_eq!(
            loader_mod.source.get_path().unwrap(),
            "maven/modrinth/NNAgCjsB/5V6jfXWM/NNAgCjsB-5V6jfXWM.jar"
        );
    }

    #[tokio::test]
    async fn lists_the_installed_mods_before_the_files() {
        let dir = std::env::temp_dir().join(format!(
            "liquidlauncher-test-{}-custom-mods",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["keystrokes-1.4.0.jar", "Zoom.jar", "notes.txt"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }

        let installed = [ModrinthMod {
            project_id: "NNAgCjsB".to_owned(),
            slug: "entityculling".to_owned(),
            title: "Entity Culling".to_owned(),
            version: "1.11.1".to_owned(),
            version_id: "5V6jfXWM".to_owned(),
        }];
        let states = HashMap::from([("keystrokes-1.4.0".to_owned(), false)]);
        let mods = custom_mods(&dir, &installed, &states, "26.2", "fabric", false)
            .await
            .unwrap();

        let rows: Vec<_> = mods
            .iter()
            .map(|row| (row.title.as_str(), row.loader_mod.enabled))
            .collect();
        assert_eq!(
            rows,
            [
                ("Entity Culling", true),
                ("keystrokes-1.4.0", false),
                ("Zoom", true)
            ]
        );
        assert!(mods[0].modrinth.as_ref().unwrap().update.is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
