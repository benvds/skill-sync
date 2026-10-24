use crate::error::SkillsSyncError;
use std::path::{Path, PathBuf};

const LOCAL_MANIFESTS: [&str; 2] = [".skills.toml", "skills.toml"];
const GLOBAL_MANIFEST_DIR: &str = "skills-sync";
const GLOBAL_MANIFEST_FILE: &str = "skills.toml";

/// Discover the manifest file from multiple locations.
///
/// Priority: --config > CWD .skills.toml > CWD skills.toml > platform config directory.
pub fn discover_manifest(config_override: Option<&Path>) -> crate::error::Result<PathBuf> {
    discover_manifest_in(Path::new("."), config_override)
}

/// Discover the manifest file relative to a base directory.
///
/// Separated from `discover_manifest` for testability (avoids CWD dependency).
pub fn discover_manifest_in(
    base_dir: &Path,
    config_override: Option<&Path>,
) -> crate::error::Result<PathBuf> {
    let mut searched = Vec::new();

    // 1. Explicit --config takes highest priority
    if let Some(path) = config_override {
        if path.exists() {
            return Ok(path.to_owned());
        }
        searched.push(path.to_owned());
        return Err(SkillsSyncError::ManifestNotFound { searched }.into());
    }

    // 2. CWD local manifest (hidden name takes precedence).
    for name in LOCAL_MANIFESTS {
        let local = base_dir.join(name);
        if local.is_file() {
            return Ok(local);
        }
        searched.push(local);
    }

    // 3. Global XDG config
    if let Some(config_dir) = dirs::config_dir() {
        let global = config_dir
            .join(GLOBAL_MANIFEST_DIR)
            .join(GLOBAL_MANIFEST_FILE);
        if global.exists() {
            return Ok(global);
        }
        searched.push(global);
    }

    Err(SkillsSyncError::ManifestNotFound { searched }.into())
}

/// Resolve paths using the manifest's location, never the caller's working directory.
/// A skills collection takes precedence over project markers (collections may be Git repos).
pub fn target_dir_for_manifest(manifest: &Path) -> PathBuf {
    let base = manifest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let logical_name = base.file_name().and_then(|s| s.to_str());
    // Canonicalize so `./.skills.toml` and symlinked skills directories work as well.
    let base = base.canonicalize().unwrap_or_else(|_| base.to_owned());
    match logical_name
        .filter(|name| matches!(*name, "skills" | ".agents" | ".codex" | ".claude"))
        .or_else(|| base.file_name().and_then(|s| s.to_str()))
    {
        Some("skills") => base,
        Some(".agents" | ".codex" | ".claude") => base.join("skills"),
        _ if is_project_dir(&base) => base.join(".agents/skills"),
        // Standalone configuration directories keep skills next to their manifest/lock.
        _ => base,
    }
}

/// Recognize a project without assuming a specific programming language or Git layout.
pub fn is_project_dir(path: &Path) -> bool {
    [
        ".git",
        ".hg",
        ".agents",
        ".codex",
        ".claude",
        "AGENTS.md",
        "CLAUDE.md",
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "Gemfile",
        "composer.json",
        "Makefile",
        "CMakeLists.txt",
        "requirements.txt",
        "setup.py",
        "deno.json",
        "deno.jsonc",
    ]
    .iter()
    .any(|marker| path.join(marker).exists())
}

pub fn global_target_dir() -> crate::error::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| SkillsSyncError::SyncFailed {
        repo: "--global".to_string(),
        reason: "cannot resolve home directory for --global target".to_string(),
    })?;
    Ok(home.join(".agents/skills"))
}
