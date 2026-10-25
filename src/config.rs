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
        Some(
            ".agents" | ".codex" | ".claude" | ".cursor" | ".gemini" | ".copilot" | ".github"
            | ".opencode" | ".kiro",
        ) => base.join("skills"),
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

/// Use XDG consistently, including macOS, while discovery retains the old platform fallback.
pub fn config_home() -> crate::error::Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()) {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(SkillsSyncError::Config("XDG_CONFIG_HOME must be absolute".into()).into());
        }
        return Ok(path);
    }
    Ok(dirs::home_dir()
        .ok_or_else(|| SkillsSyncError::Config("cannot resolve home directory".into()))?
        .join(".config"))
}

pub fn central_manifest_path() -> crate::error::Result<PathBuf> {
    Ok(config_home()?.join("skills-sync/skills.toml"))
}

pub fn find_central_manifest() -> crate::error::Result<Option<PathBuf>> {
    let mut candidates = vec![central_manifest_path()?];
    if let Some(platform) = dirs::config_dir() {
        candidates.push(platform.join("skills-sync/skills.toml"));
    }
    let old = global_target_dir()?;
    candidates.extend([old.join(".skills.toml"), old.join("skills.toml")]);
    Ok(candidates.into_iter().find(|p| p.is_file()))
}

pub fn find_project_manifest(start: &Path) -> Option<PathBuf> {
    let home = dirs::home_dir();
    for dir in start.ancestors() {
        for name in LOCAL_MANIFESTS {
            let path = dir.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
        if dir.join(".git").exists() || home.as_deref() == Some(dir) {
            break;
        }
    }
    None
}
