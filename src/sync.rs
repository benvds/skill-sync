use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::SkillsSyncError;
use crate::metadata::{write_metadata_atomic, SkillMeta};

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
    cmd.args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SkillsSyncError::GitNotFound { source: e }
        } else {
            SkillsSyncError::Io(e)
        }
    })?;

    let output = child.wait_with_output().map_err(SkillsSyncError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let args_str = args.join(" ");
        return Err(SkillsSyncError::GitFailed {
            args: args_str,
            stderr,
        }
        .into());
    }

    Ok(())
}

// ── Core sync operations ──────────────────────────────────────────────────────

/// Install a skill from GitHub into `target_dir`.
///
/// Single-skill repo: shallow clone (`--depth=1`) → atomic rename to final path.
/// Monorepo skill:    4-step sparse checkout → move subdirectory → atomic rename.
///
/// Metadata (`.skills.lock`) is written ONLY after the final rename succeeds.
/// On git failure the temp directory is removed (best-effort) before returning.
pub fn execute_install(
    repo: &str,
    subdirectory: &Option<String>,
    remote_sha: &str,
    branch: &str,
    target_dir: &Path,
    token: Option<&str>,
) -> crate::error::Result<()> {
    let dir_name = skill_dir_name(repo, subdirectory);
    let final_path = target_dir.join(&dir_name);
    let tmp_name = format!(".skills-sync-tmp-{}-{}", std::process::id(), &dir_name);
    let tmp_path = target_dir.join(&tmp_name);

    std::fs::create_dir_all(target_dir).map_err(SkillsSyncError::Io)?;

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
            .and_then(|_| {
                std::fs::rename(&tmp_path, &final_path).map_err(|e| {
                    SkillsSyncError::RenameFailed {
                        path: final_path.clone(),
                        source: e,
                    }
                    .into()
                })
            })
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
            .and_then(|_| {
                run_git(&["sparse-checkout", "init", "--cone"], Some(&tmp_path))
            })
            .and_then(|_| {
                run_git(&["sparse-checkout", "set", subdir], Some(&tmp_path))
            })
            .and_then(|_| run_git(&["checkout"], Some(&tmp_path)))
            .and_then(|_| {
                std::fs::rename(tmp_path.join(subdir), &final_path).map_err(|e| {
                    SkillsSyncError::RenameFailed {
                        path: final_path.clone(),
                        source: e,
                    }
                    .into()
                })
            });

            if result.is_ok() {
                // Clean up the temp clone (has .git and root files)
                let _ = std::fs::remove_dir_all(&tmp_path);
            }

            result
        }
    };

    if let Err(e) = git_result {
        // Best-effort cleanup of temp dir on failure
        let _ = std::fs::remove_dir_all(&tmp_path);
        return Err(e);
    }

    // Write metadata ONLY after successful rename
    let meta_path = crate::metadata::meta_path_for_skill(target_dir, &dir_name);
    let meta = match subdirectory {
        None => SkillMeta::new(repo, remote_sha),
        Some(subdir) => SkillMeta::new(repo, remote_sha).with_subdirectory(subdir.clone()),
    };
    write_metadata_atomic(&meta_path, &meta).map_err(SkillsSyncError::Io)?;

    Ok(())
}

/// Update an already-installed skill to a new SHA.
///
/// Reads the old `.skills.lock` to preserve `installed_at`, removes the old
/// directory, calls `execute_install`, then patches `installed_at` back.
pub fn execute_update(
    repo: &str,
    subdirectory: &Option<String>,
    new_sha: &str,
    branch: &str,
    target_dir: &Path,
    token: Option<&str>,
) -> crate::error::Result<()> {
    let dir_name = skill_dir_name(repo, subdirectory);
    let existing_path = target_dir.join(&dir_name);

    // Preserve installed_at from old metadata (read from sibling lock file)
    let old_meta_path = crate::metadata::meta_path_for_skill(target_dir, &dir_name);
    let installed_at = crate::metadata::read_metadata(&old_meta_path)
        .ok()
        .map(|m| m.installed_at)
        .unwrap_or_else(chrono::Utc::now);

    // Remove old directory
    std::fs::remove_dir_all(&existing_path).map_err(|e| SkillsSyncError::RemoveFailed {
        path: existing_path.clone(),
        source: e,
    })?;

    // Re-install fresh
    execute_install(repo, subdirectory, new_sha, branch, target_dir, token)?;

    // Patch installed_at back (write to sibling lock file)
    let meta_path = crate::metadata::meta_path_for_skill(target_dir, &dir_name);
    if let Ok(mut meta) = crate::metadata::read_metadata(&meta_path) {
        meta.installed_at = installed_at;
        let _ = crate::metadata::write_metadata_atomic(&meta_path, &meta);
    }

    Ok(())
}

/// Remove an installed skill directory and its sibling lock file.
///
/// Returns `RemoveFailed` if the directory cannot be removed (including if it
/// does not exist). The lock file removal is best-effort — if the lock file
/// doesn't exist or fails to delete, the skill directory removal still succeeds.
pub fn execute_remove(skill_path: &Path, lock_path: Option<&Path>) -> crate::error::Result<()> {
    std::fs::remove_dir_all(skill_path).map_err(|e| SkillsSyncError::RemoveFailed {
        path: skill_path.to_owned(),
        source: e,
    })?;
    // Best-effort lock file cleanup
    if let Some(lp) = lock_path {
        let _ = std::fs::remove_file(lp);
    }
    Ok(())
}

// ── Orphan detection and cleanup ──────────────────────────────────────────────

/// Clean up stale temp directories left by interrupted sync runs.
///
/// Scans `target_dir` for entries matching `.skills-sync-tmp-*` and removes
/// them. Errors are silently ignored (best-effort cleanup).
pub fn cleanup_stale_temp_dirs(target_dir: &Path) {
    let entries = match std::fs::read_dir(target_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(".skills-sync-tmp-") {
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
}

/// Scan `target_dir` for skill directories whose source repo is not in `manifest_repos`.
///
/// Returns an empty Vec if the directory does not exist. Entries starting with
/// `.skills-sync-tmp-` are skipped (temp dirs). Results are sorted by dir_name
/// for deterministic output.
pub fn scan_orphans(target_dir: &Path, manifest_repos: &HashSet<String>) -> Vec<OrphanSkill> {
    let mut orphans = Vec::new();

    let entries = match std::fs::read_dir(target_dir) {
        Ok(e) => e,
        Err(_) => return orphans,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip temp dirs
        if name_str.starts_with(".skills-sync-tmp-") {
            continue;
        }

        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let meta_path = crate::metadata::meta_path_for_skill(target_dir, &name_str);
        if let Ok(meta) = crate::metadata::read_metadata(&meta_path) {
            if !manifest_repos.contains(&meta.source) {
                orphans.push(OrphanSkill {
                    dir_path: path,
                    dir_name: name_str.into_owned(),
                    source: meta.source,
                });
            }
        }
    }

    orphans.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));
    orphans
}

/// Prompt the user to confirm batch removal of orphaned skills.
///
/// Prints a list of orphans to stderr, then presents a Y/n prompt.
/// Non-interactive terminals (no TTY) return false silently — no removal occurs.
fn prompt_orphan_removal(orphans: &[OrphanSkill]) -> bool {
    eprintln!("warning: {} orphaned skill(s) found:", orphans.len());
    for orphan in orphans {
        eprintln!("  {} ({})", orphan.dir_name, orphan.source);
    }

    match dialoguer::Confirm::new()
        .with_prompt("Remove orphaned skills?")
        .default(false)
        .interact_opt()
    {
        Ok(Some(true)) => true,
        _ => false,
    }
}

/// Execute the full sync plan: cleanup, installs, updates, orphan detection, and removal.
///
/// Processes actions sequentially, warning on per-skill failures and continuing.
/// After installs/updates, scans for global orphans (repos removed from manifest)
/// and combines with resolver Remove actions into a single batch prompt.
///
/// Returns `Err(SyncFailed)` if any skill operations failed; Ok(()) otherwise.
pub fn execute_sync_plan(
    actions: &[crate::resolver::SyncAction],
    target_dir: &Path,
    token: Option<&str>,
    branch_map: &HashMap<String, String>,
    manifest_repos: &HashSet<String>,
) -> crate::error::Result<()> {
    cleanup_stale_temp_dirs(target_dir);

    // Ensure target dir exists
    let _ = std::fs::create_dir_all(target_dir);

    let mut failed_skills: Vec<String> = Vec::new();
    let mut succeeded: usize = 0;
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
                match execute_install(repo, subdirectory, remote_sha, branch, target_dir, token) {
                    Ok(()) => {
                        println!("installed: {}", dir_name);
                        succeeded += 1;
                    }
                    Err(e) => {
                        eprintln!("warning: failed to install {}: {}", dir_name, e);
                        failed_skills.push(dir_name);
                    }
                }
            }
            crate::resolver::SyncAction::Update {
                repo,
                subdirectory,
                new_sha,
                ..
            } => {
                let dir_name = skill_dir_name(repo, subdirectory);
                let branch = branch_map.get(repo).map(|s| s.as_str()).unwrap_or("main");
                match execute_update(repo, subdirectory, new_sha, branch, target_dir, token) {
                    Ok(()) => {
                        println!("updated: {}", dir_name);
                        succeeded += 1;
                    }
                    Err(e) => {
                        eprintln!("warning: failed to update {}: {}", dir_name, e);
                        failed_skills.push(dir_name);
                    }
                }
            }
            crate::resolver::SyncAction::Remove { .. } => {
                // Collect for batch removal with global orphans
                remove_actions.push(action);
            }
            crate::resolver::SyncAction::Skip { .. } => {
                // No action needed
            }
        }
    }

    // Build global orphan list (repos removed from manifest entirely)
    let global_orphans = scan_orphans(target_dir, manifest_repos);

    // Convert resolver Remove actions to OrphanSkill entries
    let mut all_orphans: Vec<OrphanSkill> = Vec::new();

    for action in &remove_actions {
        if let crate::resolver::SyncAction::Remove {
            repo,
            subdirectory,
            ..
        } = action
        {
            let dir_name = skill_dir_name(repo, subdirectory);
            let dir_path = target_dir.join(&dir_name);
            all_orphans.push(OrphanSkill {
                dir_path,
                dir_name,
                source: repo.clone(),
            });
        }
    }

    // Add global orphans (skip any already covered by remove_actions)
    let remove_action_dirs: HashSet<String> = all_orphans
        .iter()
        .map(|o| o.dir_name.clone())
        .collect();
    for orphan in global_orphans {
        if !remove_action_dirs.contains(&orphan.dir_name) {
            all_orphans.push(orphan);
        }
    }

    // Sort for deterministic display
    all_orphans.sort_by(|a, b| a.dir_name.cmp(&b.dir_name));

    // Batch orphan removal with single prompt
    if !all_orphans.is_empty() {
        let confirmed = prompt_orphan_removal(&all_orphans);
        if confirmed {
            for orphan in &all_orphans {
                let lock_path = crate::metadata::meta_path_for_skill(target_dir, &orphan.dir_name);
                match execute_remove(&orphan.dir_path, Some(&lock_path)) {
                    Ok(()) => {
                        println!("removed: {}", orphan.dir_name);
                        succeeded += 1;
                    }
                    Err(e) => {
                        eprintln!("warning: failed to remove {}: {}", orphan.dir_name, e);
                        failed_skills.push(orphan.dir_name.clone());
                    }
                }
            }
        }
    }

    let failed_count = failed_skills.len();
    eprintln!(
        "sync complete: {} succeeded, {} failed",
        succeeded, failed_count
    );

    if failed_count > 0 {
        return Err(SkillsSyncError::SyncFailed {
            repo: "multiple".to_string(),
            reason: format!("{} skill(s) failed to sync", failed_count),
        }
        .into());
    }

    Ok(())
}
