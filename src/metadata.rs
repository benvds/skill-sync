use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Filename for the consolidated lock file (stored next to the manifest file).
pub const META_FILENAME: &str = ".skills.lock";

/// Consolidated lock file containing metadata for all installed skills.
///
/// Stored next to the manifest file (e.g., `.skills.lock` alongside `.skills.toml`).
/// Keys in the `skills` map are the skill_key format ("owner/repo" or "owner/repo/subdir").
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LockFile {
    pub schema_version: u32,
    pub skills: HashMap<String, SkillMeta>,
}

impl LockFile {
    /// Create a new empty LockFile with schema_version=1.
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            skills: HashMap::new(),
        }
    }
}

impl Default for LockFile {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-skill metadata tracking installation state.
///
/// Written as entries in the consolidated LockFile.
/// Fields: source, subdirectory, sha, timestamps.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SkillMeta {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,
    pub sha: String,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SkillMeta {
    /// Create a new SkillMeta with current timestamps.
    pub fn new(source: impl Into<String>, sha: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            source: source.into(),
            subdirectory: None,
            sha: sha.into(),
            installed_at: now,
            updated_at: now,
        }
    }

    /// Set the subdirectory field (for monorepo skills).
    pub fn with_subdirectory(mut self, subdir: impl Into<String>) -> Self {
        self.subdirectory = Some(subdir.into());
        self
    }
}

/// Write a LockFile atomically using temp file + rename.
///
/// The temp file is created in the same directory as the target
/// to ensure the rename is atomic (same filesystem).
pub fn write_lock_file_atomic(path: &Path, lock_file: &LockFile) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path has no parent directory",
        )
    })?;
    // Ensure parent directory exists
    std::fs::create_dir_all(dir)?;
    let mut tmp = NamedTempFile::new_in(dir)?;
    let toml_str = toml::to_string_pretty(lock_file)
        .map_err(std::io::Error::other)?;
    tmp.write_all(toml_str.as_bytes())?;
    tmp.flush()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// Read a LockFile from a TOML file.
///
/// If the file does not exist, returns `Ok(LockFile::new())` (empty, not an error).
/// This handles the first-run case where no lock file exists yet.
pub fn read_lock_file(path: &Path) -> std::io::Result<LockFile> {
    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(LockFile::new()),
        Err(e) => Err(e),
    }
}

/// Compute the lock file path for a given manifest path.
///
/// Replaces the manifest file extension with "lock":
/// `.skills.toml` → `.skills.lock`
pub fn lock_path_for_manifest(manifest_path: &Path) -> PathBuf {
    manifest_path.with_extension("lock")
}
