//! Resolve sources once, then reconcile independently tracked copies at every destination.
use crate::{
    deployment::{DeploymentLock, Installation},
    error::{Result, SkillsSyncError},
    github::GitHubClient,
    manifest::{parse_repo_entry, Manifest},
    resolver, sync,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

fn failure(path: &Path, reason: impl Into<String>) -> miette::Report {
    SkillsSyncError::SyncFailed {
        repo: path.display().to_string(),
        reason: reason.into(),
    }
    .into()
}

/// Supporting files and executable bits are part of the fingerprint. Git internals aren't.
pub fn content_hash(root: &Path) -> Result<String> {
    fn visit(root: &Path, dir: &Path, hash: &mut Sha256) -> Result<()> {
        let mut entries = std::fs::read_dir(dir)
            .map_err(SkillsSyncError::Io)?
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(SkillsSyncError::Io)?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            if entry.file_name() == ".git" {
                continue;
            }
            let path = entry.path();
            let meta = entry.metadata().map_err(SkillsSyncError::Io)?;
            if entry.file_type().map_err(SkillsSyncError::Io)?.is_symlink()
                || (!meta.is_file() && !meta.is_dir())
            {
                return Err(failure(
                    &path,
                    "skills may contain only regular files and directories",
                ));
            }
            let relative = path.strip_prefix(root).unwrap().to_string_lossy();
            hash.update((relative.len() as u64).to_le_bytes());
            hash.update(relative.as_bytes());
            if meta.is_dir() {
                hash.update(b"dir");
                visit(root, &path, hash)?;
            } else {
                hash.update(b"file");
                hash.update(meta.len().to_le_bytes());
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    hash.update((meta.permissions().mode() & 0o111).to_le_bytes());
                }
                let mut file = std::fs::File::open(&path).map_err(SkillsSyncError::Io)?;
                let mut buffer = [0u8; 8192];
                loop {
                    let count = file.read(&mut buffer).map_err(SkillsSyncError::Io)?;
                    if count == 0 {
                        break;
                    }
                    hash.update(&buffer[..count]);
                }
            }
        }
        Ok(())
    }
    if !std::fs::symlink_metadata(root)
        .map_err(SkillsSyncError::Io)?
        .file_type()
        .is_dir()
    {
        return Err(failure(
            root,
            "managed skill directory was replaced or symlinked",
        ));
    }
    let mut hash = Sha256::new();
    visit(root, root, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn copy_directory(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir(to).map_err(SkillsSyncError::Io)?;
    for entry in std::fs::read_dir(from).map_err(SkillsSyncError::Io)? {
        let entry = entry.map_err(SkillsSyncError::Io)?;
        if entry.file_name() == ".git" {
            continue;
        }
        let destination = to.join(entry.file_name());
        let kind = entry.file_type().map_err(SkillsSyncError::Io)?;
        if kind.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), destination).map_err(SkillsSyncError::Io)?;
        } else {
            return Err(failure(
                &entry.path(),
                "cannot copy symlinks or special files",
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct PreparedSkill {
    pub key: String,
    pub sha: String,
    pub directory: String,
    pub path: PathBuf,
    pub hash: String,
}
impl PreparedSkill {
    pub fn from_directory(key: String, sha: String, path: PathBuf) -> Result<Self> {
        Ok(Self {
            directory: sync::read_skill_name(&path)?,
            hash: content_hash(&path)?,
            key,
            sha,
            path,
        })
    }
}

pub struct PreparedSources {
    pub skills: Vec<PreparedSkill>,
    _checkout: tempfile::TempDir,
}
impl PreparedSources {
    pub fn fetch(manifest: &Manifest, client: &GitHubClient, token: Option<&str>) -> Result<Self> {
        let checkout = tempfile::tempdir().map_err(SkillsSyncError::Io)?;
        let mut groups: BTreeMap<String, Vec<Option<String>>> = BTreeMap::new();
        for entry in &manifest.skills.repos {
            let (owner, repo, subdir) = parse_repo_entry(entry);
            groups
                .entry(format!("{owner}/{repo}"))
                .or_default()
                .push(subdir.map(str::to_owned));
        }
        let mut skills = Vec::new();
        for (index, (slug, pins)) in groups.into_iter().enumerate() {
            let (owner, repo, _) = parse_repo_entry(&slug);
            let info = client.get_repo_info(owner, repo)?;
            let mut tree = client.get_tree(owner, repo, &info.default_branch)?;
            let root = checkout.path().join(index.to_string());
            let url = sync::build_clone_url(&slug, token);
            let sparse = pins.iter().all(Option::is_some);
            let mut clone_args = vec![
                "clone",
                "--depth=1",
                "--filter=blob:none",
                "--branch",
                &info.default_branch,
            ];
            if sparse {
                clone_args.push("--no-checkout");
            }
            clone_args.extend(["--", &url, root.to_str().unwrap()]);
            sync::run_git(&clone_args, None)?;
            if sparse {
                sync::run_git(&["sparse-checkout", "init", "--cone"], Some(&root))?;
                let mut args = vec!["sparse-checkout", "set", "--"];
                args.extend(pins.iter().filter_map(|pin| pin.as_deref()));
                sync::run_git(&args, Some(&root))?;
                sync::run_git(&["checkout"], Some(&root))?;
            }
            // The checkout is authoritative when the API is cached or truncated.
            let actual = git_output(&root, &["rev-parse", "HEAD^{tree}"])?;
            let sha = String::from_utf8(actual)
                .map_err(|e| failure(&root, e.to_string()))?
                .trim()
                .to_owned();
            if tree.truncated || sha != tree.sha {
                let listing = git_output(&root, &["ls-tree", "-rz", "HEAD"])?;
                let mut entries = Vec::new();
                for record in listing
                    .split(|byte| *byte == 0)
                    .filter(|part| !part.is_empty())
                {
                    let text =
                        std::str::from_utf8(record).map_err(|e| failure(&root, e.to_string()))?;
                    let (meta, path) = text
                        .split_once('\t')
                        .ok_or_else(|| failure(&root, "invalid git tree entry"))?;
                    let fields: Vec<_> = meta.split_whitespace().collect();
                    if fields.len() != 3 {
                        return Err(failure(&root, "invalid git tree metadata"));
                    }
                    entries.push(crate::github::TreeEntry {
                        path: path.into(),
                        entry_type: fields[1].into(),
                        sha: fields[2].into(),
                    });
                }
                tree = crate::github::TreeResponse {
                    sha,
                    tree: entries,
                    truncated: false,
                };
            }
            let mut selected = BTreeMap::new();
            for pin in pins {
                let detected = if let Some(pin) = pin {
                    vec![resolver::detect_pinned_skill(&tree, &pin).ok_or_else(|| {
                        failure(
                            Path::new(&slug),
                            format!("no SKILL.md found at {pin}/SKILL.md"),
                        )
                    })?]
                } else {
                    resolver::detect_skills(&tree)
                };
                if detected.is_empty() {
                    return Err(failure(Path::new(&slug), "no SKILL.md found"));
                }
                for skill in detected {
                    selected.insert(resolver::skill_key(&slug, &skill.subdirectory), skill);
                }
            }
            for (key, remote) in selected {
                let path = remote
                    .subdirectory
                    .map(|dir| root.join(dir))
                    .unwrap_or_else(|| root.clone());
                skills.push(PreparedSkill::from_directory(key, tree.sha.clone(), path)?);
            }
        }
        Ok(Self {
            skills,
            _checkout: checkout,
        })
    }
}

fn git_output(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(SkillsSyncError::Io)?;
    if !output.status.success() {
        return Err(failure(root, format!("git {} failed", args.join(" "))));
    }
    Ok(output.stdout)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Install,
    Update,
    Remove,
    Forget,
}
#[derive(Debug)]
pub struct Operation {
    pub action: Action,
    pub target: PathBuf,
    pub skill: String,
    pub desired: Option<usize>,
    pub previous: Option<Installation>,
    expected_hash: Option<String>,
}
#[derive(Debug, Default)]
pub struct Plan {
    pub operations: Vec<Operation>,
    pub conflicts: Vec<String>,
    pub skips: usize,
}

fn exists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

pub fn plan(skills: &[PreparedSkill], targets: &[PathBuf], lock: &DeploymentLock) -> Plan {
    let mut plan = Plan::default();
    let mut names = BTreeMap::new();
    for skill in skills {
        if let Some(other) = names.insert(&skill.directory, &skill.key) {
            plan.conflicts.push(format!(
                "{} and {} declare the same skill name `{}`",
                other, skill.key, skill.directory
            ));
        }
    }
    let desired_keys: BTreeSet<_> = skills
        .iter()
        .flat_map(|s| targets.iter().map(move |t| (&s.key, t)))
        .collect();
    for target in targets {
        for (index, skill) in skills.iter().enumerate() {
            let previous = lock
                .installations
                .iter()
                .find(|i| i.skill == skill.key && i.target == *target)
                .cloned();
            let destination = target.join(&skill.directory);
            let mut expected_hash = None;
            let mut present = false;
            if let Some(old) = &previous {
                let old_path = target.join(&old.directory);
                if exists(&old_path) {
                    present = true;
                    match content_hash(&old_path) {
                        Ok(hash) if old.content_hash.as_ref().unwrap_or(&skill.hash) == &hash => {
                            expected_hash = Some(hash)
                        }
                        Ok(_) => plan.conflicts.push(format!(
                            "locally modified skill: {} (preserved)",
                            old_path.display()
                        )),
                        Err(err) => plan
                            .conflicts
                            .push(format!("{}: {err}", old_path.display())),
                    }
                }
            }
            if exists(&destination)
                && previous
                    .as_ref()
                    .is_none_or(|old| old.directory != skill.directory)
            {
                plan.conflicts.push(format!(
                    "unmanaged name collision: {} (preserved)",
                    destination.display()
                ));
            }
            if present
                && expected_hash.as_ref() == Some(&skill.hash)
                && previous.as_ref().is_some_and(|old| {
                    old.directory == skill.directory
                        && old.sha == skill.sha
                        && old.content_hash.is_some()
                })
            {
                plan.skips += 1;
                continue;
            }
            plan.operations.push(Operation {
                action: if present {
                    Action::Update
                } else {
                    Action::Install
                },
                target: target.clone(),
                skill: skill.key.clone(),
                desired: Some(index),
                previous,
                expected_hash,
            });
        }
    }
    // Removed sources AND removed targets are reconciled, but only from this manifest's lock.
    for old in &lock.installations {
        if desired_keys.contains(&(&old.skill, &old.target)) {
            continue;
        }
        let path = old.target.join(&old.directory);
        let present = exists(&path);
        let mut expected_hash = None;
        if present {
            match content_hash(&path) {
                Ok(hash) if old.content_hash.as_ref() == Some(&hash) => expected_hash = Some(hash),
                _ => plan.conflicts.push(format!(
                    "cannot remove modified or unverified skill: {} (preserved)",
                    path.display()
                )),
            }
        }
        plan.operations.push(Operation {
            action: if present {
                Action::Remove
            } else {
                Action::Forget
            },
            target: old.target.clone(),
            skill: old.skill.clone(),
            desired: None,
            previous: Some(old.clone()),
            expected_hash,
        });
    }
    plan
}

fn apply_one(
    op: &Operation,
    skills: &[PreparedSkill],
    lock: &mut DeploymentLock,
    lock_path: &Path,
) -> Result<()> {
    if op.action == Action::Forget {
        let before = lock.clone();
        lock.installations
            .retain(|entry| !(entry.skill == op.skill && entry.target == op.target));
        if let Err(err) = lock.write(lock_path) {
            *lock = before;
            return Err(SkillsSyncError::Io(err).into());
        }
        return Ok(());
    }
    std::fs::create_dir_all(&op.target).map_err(SkillsSyncError::Io)?;
    let staging = tempfile::Builder::new()
        .prefix(".skills-sync-")
        .tempdir_in(&op.target)
        .map_err(SkillsSyncError::Io)?;
    let old_path = op
        .previous
        .as_ref()
        .map(|old| op.target.join(&old.directory));
    if let Some(expected) = &op.expected_hash {
        if content_hash(old_path.as_ref().unwrap())? != *expected {
            return Err(failure(&op.target, "skill changed after planning; retry"));
        }
    }
    let replacement = op.desired.map(|index| &skills[index]);
    if let Some(skill) = replacement {
        copy_directory(&skill.path, &staging.path().join("new"))?;
        if content_hash(&staging.path().join("new"))? != skill.hash {
            return Err(failure(&skill.path, "source changed while copying"));
        }
        let destination = op.target.join(&skill.directory);
        if exists(&destination) && old_path.as_ref() != Some(&destination) {
            return Err(failure(&destination, "destination appeared after planning"));
        }
        if op.action == Action::Install && exists(&destination) {
            return Err(failure(&destination, "destination appeared after planning"));
        }
    }
    let backup = staging.path().join("previous");
    let backed_up = matches!(op.action, Action::Update | Action::Remove);
    if backed_up {
        std::fs::rename(old_path.as_ref().unwrap(), &backup).map_err(SkillsSyncError::Io)?;
    }
    let before = lock.clone();
    let mut installed = false;
    let result = (|| -> Result<()> {
        if let Some(skill) = replacement {
            std::fs::rename(staging.path().join("new"), op.target.join(&skill.directory))
                .map_err(SkillsSyncError::Io)?;
            installed = true;
        }
        lock.installations
            .retain(|entry| !(entry.skill == op.skill && entry.target == op.target));
        if let Some(skill) = replacement.filter(|_| installed) {
            let now = Utc::now();
            lock.installations.push(Installation {
                skill: skill.key.clone(),
                target: op.target.clone(),
                directory: skill.directory.clone(),
                sha: skill.sha.clone(),
                content_hash: Some(skill.hash.clone()),
                installed_at: op.previous.as_ref().map(|p| p.installed_at).unwrap_or(now),
                updated_at: now,
            });
        }
        lock.write(lock_path).map_err(SkillsSyncError::Io)?;
        Ok(())
    })();
    if let Err(err) = result {
        if let Some(skill) = replacement.filter(|_| installed) {
            let path = op.target.join(&skill.directory);
            if exists(&path) {
                std::fs::remove_dir_all(&path).map_err(SkillsSyncError::Io)?;
            }
        }
        if backed_up {
            if let Err(restore) = std::fs::rename(&backup, old_path.unwrap()) {
                let saved = staging.keep();
                return Err(failure(
                    &saved,
                    format!("{err}; recovery failed: {restore}; previous skill preserved here"),
                ));
            }
        }
        *lock = before;
        return Err(err);
    }
    Ok(())
}

pub fn apply(
    plan: &Plan,
    skills: &[PreparedSkill],
    targets: &[PathBuf],
    lock: &mut DeploymentLock,
    lock_path: &Path,
    force: bool,
) -> Result<()> {
    if !plan.conflicts.is_empty() {
        return Err(failure(lock_path, plan.conflicts.join("\n")));
    }
    let remove = plan.operations.iter().any(|op| op.action == Action::Remove);
    let allow_remove = !remove
        || force
        || matches!(
            dialoguer::Confirm::new()
                .with_prompt("Remove managed skills no longer configured?")
                .default(false)
                .interact_opt(),
            Ok(Some(true))
        );
    let mut errors = Vec::new();
    for target in targets {
        if let Err(err) = std::fs::create_dir_all(target) {
            errors.push(format!("{}: {err}", target.display()));
        }
    }
    for op in &plan.operations {
        if op.action == Action::Remove && !allow_remove {
            continue;
        }
        match apply_one(op, skills, lock, lock_path) {
            Ok(()) => println!("{:?}: {} -> {}", op.action, op.skill, op.target.display()),
            Err(err) => errors.push(format!("{} -> {}: {err}", op.skill, op.target.display())),
        }
    }
    if !errors.is_empty() {
        return Err(failure(lock_path, errors.join("\n")));
    }
    Ok(())
}
