//! Inspect only local state. Remote discovery, revision checks and baseline
//! adoption require a sync; an offline snapshot must not claim to do them.
use crate::{deployment::DeploymentLock, engine, manifest};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Unchanged,
    Modified,
    Missing,
    Unverified,
    Unmanaged,
    Unreadable,
}

#[derive(Debug, Serialize)]
pub struct Entry {
    pub skill: Option<String>,
    pub target: PathBuf,
    pub directory: String,
    pub state: State,
    pub managed: bool,
    pub selected: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub upstream_checked: bool,
    pub attention_required: bool,
    pub unresolved_sources: Vec<String>,
    pub entries: Vec<Entry>,
}

fn selects(source: &str, key: &str) -> bool {
    let (source, _) = manifest::split_revision(source);
    key == source
        || key
            .strip_prefix(source)
            .is_some_and(|rest| rest.starts_with('/'))
}

pub fn inspect(sources: &[String], targets: &[PathBuf], lock: &DeploymentLock) -> Snapshot {
    let selected = |key: &str| sources.iter().any(|source| selects(source, key));
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for installation in &lock.installations {
        let path = installation.target.join(&installation.directory);
        let (state, detail) = match std::fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (State::Missing, None),
            Err(error) => (State::Unreadable, Some(error.to_string())),
            Ok(_) => match engine::content_hash(&path) {
                Ok(hash) => match &installation.content_hash {
                    Some(expected) if expected == &hash => (State::Unchanged, None),
                    Some(_) => (State::Modified, None),
                    None => (State::Unverified, Some("legacy lock has no content baseline; sync must verify it against the source".into())),
                },
                Err(error) => (State::Unreadable, Some(error.to_string())),
            },
        };
        seen.insert((installation.target.clone(), installation.directory.clone()));
        entries.push(Entry {
            skill: Some(installation.skill.clone()),
            target: installation.target.clone(),
            directory: installation.directory.clone(),
            state,
            managed: true,
            selected: selected(&installation.skill) && targets.contains(&installation.target),
            detail,
        });
    }
    // Known skills must also be present at newly selected destinations. A name
    // already there without ownership is a collision, even if its bytes match.
    for installation in lock
        .installations
        .iter()
        .filter(|item| selected(&item.skill))
    {
        for target in targets {
            if !seen.insert((target.clone(), installation.directory.clone())) {
                continue;
            }
            let (state, detail) =
                match std::fs::symlink_metadata(target.join(&installation.directory)) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        (State::Missing, None)
                    }
                    Err(error) => (State::Unreadable, Some(error.to_string())),
                    Ok(_) => (
                        State::Unmanaged,
                        Some("selected skill name exists without an ownership record".into()),
                    ),
                };
            entries.push(Entry {
                skill: Some(installation.skill.clone()),
                target: target.clone(),
                directory: installation.directory.clone(),
                state,
                managed: false,
                selected: true,
                detail,
            });
        }
    }
    // Other skills are informational; they remain outside this manifest's ownership.
    for target in targets {
        match std::fs::read_dir(target) {
            Ok(children) => {
                for child in children {
                    match child {
                        Ok(child) => {
                            let directory = child.file_name().to_string_lossy().into_owned();
                            if child.path().join("SKILL.md").is_file()
                                && seen.insert((target.clone(), directory.clone()))
                            {
                                entries.push(Entry {
                                    skill: None,
                                    target: target.clone(),
                                    directory,
                                    state: State::Unmanaged,
                                    managed: false,
                                    selected: false,
                                    detail: None,
                                });
                            }
                        }
                        Err(error) => entries.push(unreadable_target(target, error)),
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => entries.push(unreadable_target(target, error)),
        }
    }
    entries.sort_by(|a, b| (&a.target, &a.directory).cmp(&(&b.target, &b.directory)));
    let unresolved_sources: Vec<_> = sources
        .iter()
        .filter(|source| {
            !lock
                .installations
                .iter()
                .any(|item| selects(source, &item.skill))
        })
        .cloned()
        .collect();
    let attention_required = !unresolved_sources.is_empty()
        || entries.iter().any(|entry| {
            entry.state == State::Unreadable
                || (entry.managed && !entry.selected)
                || (entry.selected && entry.state != State::Unchanged)
        });
    Snapshot {
        upstream_checked: false,
        attention_required,
        unresolved_sources,
        entries,
    }
}

fn unreadable_target(target: &Path, error: std::io::Error) -> Entry {
    Entry {
        skill: None,
        target: target.to_owned(),
        directory: String::new(),
        state: State::Unreadable,
        managed: false,
        selected: false,
        detail: Some(error.to_string()),
    }
}
