use crate::error::SkillSyncError;
use std::path::{Path, PathBuf};

const LOCAL_MANIFEST: &str = ".skills.toml";
const GLOBAL_MANIFEST_DIR: &str = "skill-sync";
const GLOBAL_MANIFEST_FILE: &str = "skills.toml";

/// Discover the manifest file from multiple locations.
///
/// Priority: --config override > CWD .skills.toml > global ~/.config/skill-sync/skills.toml
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
        return Err(SkillSyncError::ManifestNotFound { searched }.into());
    }

    // 2. CWD local manifest
    let local = base_dir.join(LOCAL_MANIFEST);
    if local.exists() {
        return Ok(local);
    }
    searched.push(local);

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

    Err(SkillSyncError::ManifestNotFound { searched }.into())
}
