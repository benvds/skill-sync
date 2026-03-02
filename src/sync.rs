use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::SkillsSyncError;
use crate::metadata::{write_metadata_atomic, SkillMeta, META_FILENAME};

// ── Directory naming ──────────────────────────────────────────────────────────

/// Convert a GitHub `owner/repo` slug (and optional subdirectory) to a
/// filesystem-safe flat directory name using `--` as separator.
///
/// Examples:
///   `owner/repo`, `None`       → `owner--repo`
///   `owner/repo`, `Some(sub)`  → `owner--repo--sub`
pub fn skill_dir_name(repo: &str, subdirectory: &Option<String>) -> String {
    let base = repo.replace('/', "--");
    match subdirectory {
        Some(subdir) => format!("{base}--{subdir}"),
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
    let meta_path = final_path.join(META_FILENAME);
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

    // Preserve installed_at from old metadata
    let old_meta_path = existing_path.join(crate::metadata::META_FILENAME);
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

    // Patch installed_at back
    let meta_path = target_dir.join(&dir_name).join(crate::metadata::META_FILENAME);
    if let Ok(mut meta) = crate::metadata::read_metadata(&meta_path) {
        meta.installed_at = installed_at;
        let _ = crate::metadata::write_metadata_atomic(&meta_path, &meta);
    }

    Ok(())
}

/// Remove an installed skill directory.
///
/// Returns `RemoveFailed` if the directory cannot be removed (including if it
/// does not exist).
pub fn execute_remove(skill_path: &Path) -> crate::error::Result<()> {
    std::fs::remove_dir_all(skill_path).map_err(|e| {
        SkillsSyncError::RemoveFailed {
            path: skill_path.to_owned(),
            source: e,
        }
        .into()
    })
}
