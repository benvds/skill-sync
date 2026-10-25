use crate::{
    error::{Result, SkillsSyncError},
    manifest::{Manifest, Scope},
};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

pub fn validate_agent(agent: &str) -> Result<()> {
    match agent {
        "codex" | "claude-code" | "cursor" | "gemini-cli" | "copilot" |
        "github-copilot" | "opencode" | "goose" | "kiro" | "universal" => Ok(()),
        _ => Err(SkillsSyncError::Config(format!("unknown agent `{agent}`; choose codex, claude-code, cursor, gemini-cli, copilot, opencode, goose, kiro or universal")).into()),
    }
}

/// Normalize existing ancestors too, so aliases/symlinks produce one lock identity.
pub fn absolute_path(path: &Path, base: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(part),
        }
    }
    let mut ancestor = normalized.as_path();
    let mut missing = Vec::new();
    while !ancestor.exists() {
        if let Some(name) = ancestor.file_name() {
            missing.push(name.to_owned());
        }
        ancestor = ancestor
            .parent()
            .ok_or_else(|| SkillsSyncError::Config("cannot resolve target path".into()))?;
    }
    let mut resolved = ancestor.canonicalize().map_err(SkillsSyncError::Io)?;
    for name in missing.iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

pub fn expand_path(value: &str, home: &Path, base: &Path) -> Result<PathBuf> {
    if value.trim().is_empty() {
        return Err(SkillsSyncError::Config("empty target directory".into()).into());
    }
    let path = if value == "~" {
        home.to_owned()
    } else if let Some(relative) = value.strip_prefix("~/") {
        home.join(relative)
    } else {
        PathBuf::from(value)
    };
    absolute_path(&path, base)
}

pub fn target_directories(
    manifest: &Manifest,
    manifest_path: &Path,
    scope: Scope,
    selected: &[String],
    home: &Path,
    config_home: &Path,
) -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir().map_err(SkillsSyncError::Io)?;
    let base = absolute_path(
        manifest_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
        &cwd,
    )?;
    let configured = manifest
        .sync
        .as_ref()
        .map(|s| s.agents.as_slice())
        .unwrap_or(&[]);
    let agents = if selected.is_empty() {
        configured
    } else {
        selected
    };
    if agents.is_empty() {
        return Ok(vec![if scope == Scope::User {
            absolute_path(&home.join(".agents/skills"), &cwd)?
        } else {
            crate::config::target_dir_for_manifest(manifest_path)
        }]);
    }
    let mut targets = BTreeSet::new();
    for agent in agents {
        validate_agent(agent)?;
        let overrides = manifest.agents.get(agent);
        let custom = overrides.and_then(|p| match scope {
            Scope::User => p.user_dir.as_deref(),
            Scope::Project => p.project_dir.as_deref(),
        });
        let target = if let Some(path) = custom {
            expand_path(path, home, &base)?
        } else {
            let (user, project) = match agent.as_str() {
                "codex" | "goose" | "universal" => (home.join(".agents/skills"), ".agents/skills"),
                "claude-code" => (home.join(".claude/skills"), ".claude/skills"),
                "cursor" => (home.join(".cursor/skills"), ".cursor/skills"),
                "gemini-cli" => (home.join(".gemini/skills"), ".gemini/skills"),
                "copilot" | "github-copilot" => (home.join(".copilot/skills"), ".github/skills"),
                "opencode" => (config_home.join("opencode/skills"), ".opencode/skills"),
                "kiro" => (home.join(".kiro/skills"), ".kiro/skills"),
                _ => unreachable!(),
            };
            absolute_path(
                &match scope {
                    Scope::User => user,
                    Scope::Project => base.join(project),
                },
                &cwd,
            )?
        };
        if scope == Scope::Project && !target.starts_with(&base) {
            return Err(SkillsSyncError::Config(format!(
                "project target {} must stay inside {}",
                target.display(),
                base.display()
            ))
            .into());
        }
        targets.insert(target);
    }
    Ok(targets.into_iter().collect())
}
