use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Filename for per-skill lock files (legacy, kept for backward compatibility).
pub const META_FILENAME: &str = ".skills.lock";

/// File extension for per-skill lock files (stored as siblings to skill directories).
pub const META_EXTENSION: &str = "lock";

/// Compute the metadata lock file path for a skill, stored as a sibling
/// to the skill directory at the target_dir level.
///
/// Example: for skill dir "owner--repo" in target_dir ".claude/skills/",
/// returns ".claude/skills/owner--repo.lock"
pub fn meta_path_for_skill(target_dir: &Path, dir_name: &str) -> PathBuf {
    target_dir.join(format!("{}.{}", dir_name, META_EXTENSION))
}

/// Per-skill metadata tracking installation state.
///
/// Written atomically to prevent corruption on interruption.
/// Fields match PRD spec: source, subdirectory, sha, timestamps.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillMeta {
    pub schema_version: u32,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,
    pub sha: String,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SkillMeta {
    /// Create a new SkillMeta with schema_version=1 and current timestamps.
    pub fn new(source: impl Into<String>, sha: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            schema_version: 1,
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

/// Write metadata atomically using temp file + rename.
///
/// The temp file is created in the same directory as the target
/// to ensure the rename is atomic (same filesystem).
pub fn write_metadata_atomic(path: &Path, meta: &SkillMeta) -> std::io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path has no parent directory",
        )
    })?;
    let mut tmp = NamedTempFile::new_in(dir)?;
    let toml_str = toml::to_string_pretty(meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    tmp.write_all(toml_str.as_bytes())?;
    tmp.flush()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// Read metadata from a TOML lock file.
pub fn read_metadata(path: &Path) -> std::io::Result<SkillMeta> {
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
