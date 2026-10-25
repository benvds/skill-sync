use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use owo_colors::{OwoColorize, Stream};

use crate::error::SkillSyncError;
use crate::metadata::{read_lock_file, write_lock_file_atomic, LockFile, SkillMeta};
use crate::resolver::skill_key;

// ── Directory naming ──────────────────────────────────────────────────────────

/// Convert a GitHub `owner/repo` slug (and optional subdirectory) to a
/// filesystem-safe flat directory name using `--` as separator.
///
/// Multi-segment subdirectory paths are flattened: slashes within the subdir
/// are also replaced with `--` to produce a flat directory name.
///
/// Examples:
///   `owner/repo`, `None`                   → `owner--repo`
///   `owner/repo`, `Some("sub")`            → `owner--repo--sub`
///   `owner/repo`, `Some("skills/creator")` → `owner--repo--skills--creator`
pub fn skill_dir_name(repo: &str, subdirectory: &Option<String>) -> String {
    let base = repo.replace('/', "--");
    match subdirectory {
        Some(subdir) => format!("{base}--{}", subdir.replace('/', "--")),
        None => base,
    }
}

// ── Clone URL builder ─────────────────────────────────────────────────────────

/// Build an HTTPS GitHub clone URL, optionally embedding an auth token.
///
/// With token:    `https://TOKEN@github.com/owner/repo.git`
/// Without token: `https://github.com/owner/repo.git`
pub fn build_clone_url(repo: &str, token: Option<&str>) -> String {
    match token {
        Some(tok) => format!("https://{tok}@github.com/{repo}.git"),
        None => format!("https://github.com/{repo}.git"),
    }
}

// ── Git subprocess runner ─────────────────────────────────────────────────────

/// Run a git command, optionally in the given working directory.
///
/// Stdout is suppressed; stderr is captured for error messages.
/// Returns `GitNotFound` if the git binary cannot be found,
/// `GitFailed` for non-zero exit codes.
pub fn run_git(args: &[&str], cwd: Option<&Path>) -> crate::error::Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::piped());

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SkillSyncError::GitNotFound { source: e }
        } else {
            SkillSyncError::Io(e)
        }
    })?;

    let output = child.wait_with_output().map_err(SkillSyncError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let args_str = args.join(" ");
        return Err(SkillSyncError::GitFailed {
            args: args_str,
            stderr,
        }
        .into());
    }

    Ok(())
}

// ── Core sync operations ──────────────────────────────────────────────────────

/// Read the portable skill name without changing the upstream SKILL.md.
pub fn read_skill_name(skill_dir: &Path) -> crate::error::Result<String> {
    #[derive(serde::Deserialize)]
    struct Frontmatter {
        name: String,
        description: String,
    }
    let invalid = |reason: String| SkillSyncError::SyncFailed {
        repo: skill_dir.display().to_string(),
        reason,
    };
    let content =
        std::fs::read_to_string(skill_dir.join("SKILL.md")).map_err(SkillSyncError::Io)?;
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return Err(invalid("SKILL.md must start with YAML frontmatter".into()).into());
    }
    let mut yaml = String::new();
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return Err(invalid("SKILL.md frontmatter has no closing ---".into()).into());
    }
    let meta: Frontmatter = serde_yaml::from_str(&yaml)
        .map_err(|e| invalid(format!("invalid SKILL.md frontmatter: {e}")))?;
    if meta.name.is_empty()
        || meta.name.len() > 64
        || !meta
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || meta.name.starts_with('-')
        || meta.name.ends_with('-')
        || meta.name.contains("--")
    {
        return Err(invalid(
            "SKILL.md name must be 1-64 lowercase letters, digits or single hyphens".into(),
        )
        .into());
    }
    if meta.description.trim().is_empty() || meta.description.chars().count() > 1024 {
        return Err(invalid("SKILL.md description must contain 1-1024 characters".into()).into());
    }
    Ok(meta.name)
}

fn install_staged_skill(staged: &Path, target: &Path) -> crate::error::Result<String> {
    let name = read_skill_name(staged)?;
    let destination = target.join(&name);
    if destination.symlink_metadata().is_ok() {
        return Err(SkillSyncError::SyncFailed {
            repo: name,
            reason: format!("destination already exists: {}", destination.display()),
        }
        .into());
    }
    std::fs::rename(staged, &destination).map_err(|source| SkillSyncError::RenameFailed {
        path: destination,
        source,
    })?;
    Ok(name)
}

/// Install a skill from GitHub into `target_dir`.
///
/// Single-skill repo: shallow clone (`--depth=1`) → atomic rename to final path.
/// Monorepo skill:    4-step sparse checkout → move subdirectory → atomic rename.
///
/// After successful install, inserts metadata into `lock_file` and writes
/// the consolidated lock file atomically to `lock_path`.
/// On git failure the temp directory is removed (best-effort) before returning.
#[allow(clippy::too_many_arguments)]
pub fn execute_install(
    repo: &str,
    subdirectory: &Option<String>,
    remote_sha: &str,
    branch: &str,
    target_dir: &Path,
    token: Option<&str>,
    lock_path: &Path,
    lock_file: &mut LockFile,
) -> crate::error::Result<()> {
    let dir_name = skill_dir_name(repo, subdirectory);
    let tmp_name = format!(".skill-sync-tmp-{}-{}", std::process::id(), &dir_name);
    let tmp_path = target_dir.join(&tmp_name);

    std::fs::create_dir_all(target_dir).map_err(SkillSyncError::Io)?;

    let clone_url = build_clone_url(repo, token);
    let tmp_path_str = tmp_path.to_string_lossy().into_owned();

    let git_result = match subdirectory {
        None => {
            // Single-skill: shallow clone directly
            run_git(
                &[
                    "clone",
                    "--depth=1",
                    "-b",
                    branch,
                    "--",
                    &clone_url,
                    &tmp_path_str,
                ],
                None,
            )
            .and_then(|_| install_staged_skill(&tmp_path, target_dir))
        }
        Some(subdir) => {
            // Monorepo: 4-step sparse checkout
            let result = run_git(
                &[
                    "clone",
                    "--depth=1",
                    "--no-checkout",
                    "--filter=blob:none",
                    "--",
                    &clone_url,
                    &tmp_path_str,
                ],
                None,
            )
            .and_then(|_| run_git(&["sparse-checkout", "init", "--cone"], Some(&tmp_path)))
            .and_then(|_| run_git(&["sparse-checkout", "set", subdir], Some(&tmp_path)))
            .and_then(|_| run_git(&["checkout"], Some(&tmp_path)))
            .and_then(|_| install_staged_skill(&tmp_path.join(subdir), target_dir));

            if result.is_ok() {
                // Clean up the temp clone (has .git and root files)
                let _ = std::fs::remove_dir_all(&tmp_path);
            }

            result
        }
    };

    let directory = match git_result {
        Ok(name) => name,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp_path);
            return Err(e);
        }
    };

    // Write metadata into the consolidated lock file ONLY after successful rename
    let mut meta = match subdirectory {
        None => SkillMeta::new(repo, remote_sha),
        Some(subdir) => SkillMeta::new(repo, remote_sha).with_subdirectory(subdir.clone()),
    };
    meta.directory = Some(directory);
    let key = skill_key(repo, subdirectory);
    lock_file.skills.insert(key, meta);
    write_lock_file_atomic(lock_path, lock_file).map_err(SkillSyncError::Io)?;

    Ok(())
}

/// Update an already-installed skill to a new SHA.
///
/// Preserves `installed_at` and restores the existing directory if installation fails.
#[allow(clippy::too_many_arguments)]
pub fn execute_update(
    repo: &str,
    subdirectory: &Option<String>,
    new_sha: &str,
    branch: &str,
    target_dir: &Path,
    token: Option<&str>,
    lock_path: &Path,
    lock_file: &mut LockFile,
) -> crate::error::Result<()> {
    let key = skill_key(repo, subdirectory);
    let dir_name = lock_file
        .skills
        .get(&key)
        .map(|meta| meta.directory_name())
        .unwrap_or_else(|| skill_dir_name(repo, subdirectory));
    let existing_path = target_dir.join(&dir_name);

    // Preserve installed_at from existing lock_file entry
    let installed_at = lock_file
        .skills
        .get(&key)
        .map(|m| m.installed_at)
        .unwrap_or_else(chrono::Utc::now);

    // Keep the old skill until the replacement succeeds, including during legacy migration.
    let backup = tempfile::tempdir_in(target_dir).map_err(SkillSyncError::Io)?;
    let backup_path = backup.path().join("skill");
    std::fs::rename(&existing_path, &backup_path).map_err(SkillSyncError::Io)?;
    let old_lock = lock_file.clone();
    if let Err(err) = execute_install(
        repo,
        subdirectory,
        new_sha,
        branch,
        target_dir,
        token,
        lock_path,
        lock_file,
    ) {
        // A lock write may fail after the new directory has already been installed.
        if lock_file != &old_lock {
            if let Some(meta) = lock_file.skills.get(&key) {
                std::fs::remove_dir_all(target_dir.join(meta.directory_name()))
                    .map_err(SkillSyncError::Io)?;
            }
        }
        std::fs::rename(&backup_path, &existing_path).map_err(SkillSyncError::Io)?;
        *lock_file = old_lock;
        return Err(err);
    }

    // Patch installed_at back and re-write the lock file
    if let Some(meta) = lock_file.skills.get_mut(&key) {
        meta.installed_at = installed_at;
    }
    write_lock_file_atomic(lock_path, lock_file).map_err(SkillSyncError::Io)?;

    Ok(())
}

/// Remove an installed skill directory and update the consolidated lock file.
///
/// Returns `RemoveFailed` if the directory cannot be removed (including if it
/// does not exist). The lock file entry removal and write are performed after
/// successful directory removal.
pub fn execute_remove(
    skill_path: &Path,
    lock_path: &Path,
    lock_file: &mut LockFile,
    skill_key_str: &str,
) -> crate::error::Result<()> {
    std::fs::remove_dir_all(skill_path).map_err(|e| SkillSyncError::RemoveFailed {
        path: skill_path.to_owned(),
        source: e,
    })?;
    // Remove entry from consolidated lock file
    lock_file.skills.remove(skill_key_str);
    // Write updated lock file
    write_lock_file_atomic(lock_path, lock_file).map_err(SkillSyncError::Io)?;
    Ok(())
}

// ── Orphan detection and cleanup ──────────────────────────────────────────────

/// Clean up stale temp directories left by interrupted sync runs.
///
/// Scans `target_dir` for entries matching `.skill-sync-tmp-*` and removes
/// them. Errors are silently ignored (best-effort cleanup).
pub fn cleanup_stale_temp_dirs(target_dir: &Path) {
    let entries = match std::fs::read_dir(target_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(".skill-sync-tmp-") {
            eprintln!("cleaned up stale temp dir: {}", name_str);
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// An orphaned skill directory found in the target dir but not in the manifest.
pub struct OrphanSkill {
    pub dir_path: PathBuf,
    pub dir_name: String,
    pub source: String,
    pub skill_key: String,
}

/// Scan the consolidated lock file for skill entries whose source repo is not in `manifest_repos`
/// and whose directory still exists on disk.
///
/// Returns an empty Vec if no orphans found. Results are sorted by dir_name
/// for deterministic output.
pub fn scan_orphans(
    target_dir: &Path,
    manifest_repos: &HashSet<String>,
    lock_file: &LockFile,
) -> Vec<OrphanSkill> {
    let mut orphans = Vec::new();

    for (key, meta) in &lock_file.skills {
        if manifest_repos.contains(&meta.source) {
            continue;
        }

        // Build the directory path from skill key
        let dir_name = meta.directory_name();
        let dir_path = target_dir.join(&dir_name);

        // Only report as orphan if directory actually exists on disk
        if dir_path.is_dir() {
            orphans.push(OrphanSkill {
                dir_path,
                dir_name,
                source: meta.source.clone(),
                skill_key: key.clone(),
            });
        }
    }

    orphans.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    orphans
}

/// Prompt the user to confirm batch removal of orphaned skills.
///
/// When `force` is true, auto-confirms without user interaction (for CI usage).
/// Prints a list of orphans to stderr, then presents a Y/n prompt.
/// Non-interactive terminals (no TTY) return false silently — no removal occurs.
fn prompt_orphan_removal(orphans: &[OrphanSkill], force: bool) -> bool {
    if force {
        return true;
    }

    eprintln!(
        "{} {} orphaned skill(s) found:",
        "warning:".if_supports_color(Stream::Stderr, |t| t.yellow()),
        orphans.len()
    );
    for orphan in orphans {
        eprintln!("  {} ({})", orphan.dir_name, orphan.source);
    }

    matches!(
        dialoguer::Confirm::new()
            .with_prompt("Remove orphaned skills?")
            .default(false)
            .interact_opt(),
        Ok(Some(true))
    )
}

/// Execute the full sync plan: cleanup, installs, updates, orphan detection, and removal.
///
/// Reads the consolidated lock file at `lock_path` at the start, passes it through
/// all install/update/remove operations, writing it after each operation for crash safety.
/// After installs/updates, scans the lock file for global orphans (repos removed from manifest)
/// and combines with resolver Remove actions into a single batch prompt.
///
/// When `verbose` is true, prints per-skill SHA details to stderr before each operation:
/// installs show target SHA, updates show old..new SHA comparison, skips show current SHA.
///
/// Returns `Err(SyncFailed)` if any skill operations failed; Ok(()) otherwise.
pub fn execute_sync_plan(
    actions: &[crate::resolver::SyncAction],
    target_dir: &Path,
    token: Option<&str>,
    branch_map: &HashMap<String, String>,
    manifest_repos: &HashSet<String>,
    lock_path: &Path,
    force: bool,
    verbose: bool,
) -> crate::error::Result<()> {
    cleanup_stale_temp_dirs(target_dir);

    // Ensure target dir exists
    std::fs::create_dir_all(target_dir).map_err(SkillSyncError::Io)?;

    // Read consolidated lock file (returns empty if not found)
    let mut lock_file = read_lock_file(lock_path).map_err(SkillSyncError::Io)?;

    let mut failed_skills: Vec<String> = Vec::new();
    let mut install_count: usize = 0;
    let mut update_count: usize = 0;
    let mut remove_count: usize = 0;
    let mut remove_actions: Vec<&crate::resolver::SyncAction> = Vec::new();

    for action in actions {
        match action {
            crate::resolver::SyncAction::Install {
                repo,
                subdirectory,
                remote_sha,
            } => {
                let dir_name = skill_dir_name(repo, subdirectory);
                let branch = branch_map.get(repo).map(|s| s.as_str()).unwrap_or("main");
                if verbose {
                    eprintln!(
                        "  {} -> {}",
                        dir_name,
                        &remote_sha[..8.min(remote_sha.len())]
                    );
                }
                match execute_install(
                    repo,
                    subdirectory,
                    remote_sha,
                    branch,
                    target_dir,
                    token,
                    lock_path,
                    &mut lock_file,
                ) {
                    Ok(()) => {
                        println!(
                            "{} {}",
                            "installed:".if_supports_color(Stream::Stdout, |t| t.green()),
                            lock_file.skills[&skill_key(repo, subdirectory)].directory_name()
                        );
                        install_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "{} {}: {}",
                            "failed:".if_supports_color(Stream::Stderr, |t| t.red()),
                            dir_name,
                            e
                        );
                        failed_skills.push(dir_name);
                    }
                }
            }
            crate::resolver::SyncAction::Update {
                repo,
                subdirectory,
                old_sha,
                new_sha,
            } => {
                let dir_name = skill_dir_name(repo, subdirectory);
                let branch = branch_map.get(repo).map(|s| s.as_str()).unwrap_or("main");
                if verbose {
                    eprintln!(
                        "  {} {} -> {}",
                        dir_name,
                        &old_sha[..8.min(old_sha.len())],
                        &new_sha[..8.min(new_sha.len())]
                    );
                }
                match execute_update(
                    repo,
                    subdirectory,
                    new_sha,
                    branch,
                    target_dir,
                    token,
                    lock_path,
                    &mut lock_file,
                ) {
                    Ok(()) => {
                        println!(
                            "{} {}",
                            "updated:".if_supports_color(Stream::Stdout, |t| t.yellow()),
                            lock_file.skills[&skill_key(repo, subdirectory)].directory_name()
                        );
                        update_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "{} {}: {}",
                            "failed:".if_supports_color(Stream::Stderr, |t| t.red()),
                            dir_name,
                            e
                        );
                        failed_skills.push(dir_name);
                    }
                }
            }
            crate::resolver::SyncAction::Remove { .. } => {
                // Collect for batch removal with global orphans
                remove_actions.push(action);
            }
            crate::resolver::SyncAction::Skip {
                repo,
                subdirectory,
                sha,
            } => {
                if verbose {
                    let dir_name = skill_dir_name(repo, subdirectory);
                    eprintln!("  skip: {} ({})", dir_name, &sha[..8.min(sha.len())]);
                }
            }
        }
    }

    // Build global orphan list (repos removed from manifest entirely)
    let global_orphans = scan_orphans(target_dir, manifest_repos, &lock_file);

    // Convert resolver Remove actions to OrphanSkill entries
    let mut all_orphans: Vec<OrphanSkill> = Vec::new();

    for action in &remove_actions {
        if let crate::resolver::SyncAction::Remove {
            repo, subdirectory, ..
        } = action
        {
            let key = skill_key(repo, subdirectory);
            let dir_name = lock_file
                .skills
                .get(&key)
                .map(|meta| meta.directory_name())
                .unwrap_or_else(|| skill_dir_name(repo, subdirectory));
            let dir_path = target_dir.join(&dir_name);
            all_orphans.push(OrphanSkill {
                dir_path,
                dir_name,
                source: repo.clone(),
                skill_key: key,
            });
        }
    }

    // Add global orphans (skip any already covered by remove_actions)
    let remove_action_dirs: HashSet<String> =
        all_orphans.iter().map(|o| o.dir_name.clone()).collect();
    for orphan in global_orphans {
        if !remove_action_dirs.contains(&orphan.dir_name) {
            all_orphans.push(orphan);
        }
    }

    // Sort for deterministic display
    all_orphans.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));

    // Batch orphan removal with single prompt
    if !all_orphans.is_empty() {
        let confirmed = prompt_orphan_removal(&all_orphans, force);
        if confirmed {
            for orphan in &all_orphans {
                match execute_remove(
                    &orphan.dir_path,
                    lock_path,
                    &mut lock_file,
                    &orphan.skill_key,
                ) {
                    Ok(()) => {
                        println!(
                            "{} {}",
                            "removed:".if_supports_color(Stream::Stdout, |t| t.green()),
                            orphan.dir_name
                        );
                        remove_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "{} {}: {}",
                            "failed:".if_supports_color(Stream::Stderr, |t| t.red()),
                            orphan.dir_name,
                            e
                        );
                        failed_skills.push(orphan.dir_name.clone());
                    }
                }
            }
        }
    }

    let failed_count = failed_skills.len();
    let summary = format!(
        "sync complete: {} installed, {} updated, {} removed",
        install_count, update_count, remove_count
    );
    if failed_count > 0 {
        println!("{}", summary.if_supports_color(Stream::Stdout, |t| t.red()));
        return Err(SkillSyncError::SyncFailed {
            repo: "multiple".to_string(),
            reason: format!("{} skill(s) failed to sync", failed_count),
        }
        .into());
    } else {
        println!(
            "{}",
            summary.if_supports_color(Stream::Stdout, |t| t.green())
        );
    }

    Ok(())
}
