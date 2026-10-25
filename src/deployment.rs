use crate::metadata::LockFile;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
pub struct DeploymentLock {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<crate::manifest::Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<PathBuf>,
    #[serde(default)]
    pub installations: Vec<Installation>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Installation {
    pub skill: String,
    pub target: PathBuf,
    pub directory: String,
    pub sha: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn safe_directory(value: &str) -> bool {
    !value.is_empty() && value != "." && value != ".." && !value.contains(['/', '\\'])
}

impl DeploymentLock {
    pub fn read(path: &Path, legacy_target: &Path) -> io::Result<Self> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(Self {
                    schema_version: 2,
                    ..Self::default()
                })
            }
            Err(e) => return Err(e),
        };
        let table: toml::Table = toml::from_str(&content).map_err(io::Error::other)?;
        let mut lock = match table.get("schema_version").and_then(|v| v.as_integer()) {
            Some(1) => {
                let old: LockFile = toml::from_str(&content).map_err(io::Error::other)?;
                Self {
                    schema_version: 2,
                    installations: old
                        .skills
                        .into_iter()
                        .map(|(skill, meta)| Installation {
                            skill,
                            target: legacy_target.to_owned(),
                            directory: meta.directory_name(),
                            sha: meta.sha,
                            content_hash: None,
                            installed_at: meta.installed_at,
                            updated_at: meta.updated_at,
                        })
                        .collect(),
                    ..Self::default()
                }
            }
            Some(2) => toml::from_str(&content).map_err(io::Error::other)?,
            _ => {
                return Err(io::Error::other(
                    "unsupported skills lock schema_version (expected 1 or 2)",
                ))
            }
        };
        let mut seen = std::collections::HashSet::new();
        let mut destinations = std::collections::HashSet::new();
        for entry in &lock.installations {
            if !entry.target.is_absolute()
                || entry
                    .target
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
                || !safe_directory(&entry.directory)
                || !seen.insert((&entry.skill, &entry.target))
                || !destinations.insert((&entry.directory, &entry.target))
            {
                return Err(io::Error::other(
                    "invalid or duplicate skill/target entry in lock file",
                ));
            }
        }
        lock.sort();
        Ok(lock)
    }

    /// Committed project locks must never reconcile targets in another checkout/worktree.
    pub fn bind_scope(&mut self, scope: crate::manifest::Scope, root: &Path) -> io::Result<()> {
        use crate::manifest::Scope;
        if self.scope.is_some_and(|old| old != scope) {
            return Err(io::Error::other(
                "manifest scope changed; keep separate user/project manifests and locks",
            ));
        }
        if scope == Scope::Project {
            let previous = self.project_root.as_deref().unwrap_or(root);
            for entry in &mut self.installations {
                let relative = entry.target.strip_prefix(previous).map_err(|_| {
                    io::Error::other("project lock contains a target outside the project")
                })?;
                entry.target = root.join(relative);
            }
            self.project_root = Some(root.to_owned());
        }
        self.scope = Some(scope);
        Ok(())
    }

    pub fn sort(&mut self) {
        self.installations
            .sort_by(|a, b| (&a.target, &a.skill).cmp(&(&b.target, &b.skill)));
    }

    pub fn write(&mut self, path: &Path) -> io::Result<()> {
        self.schema_version = 2;
        self.sort();
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir)?;
        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
        temp.write_all(
            toml::to_string_pretty(self)
                .map_err(io::Error::other)?
                .as_bytes(),
        )?;
        temp.flush()?;
        temp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
}
