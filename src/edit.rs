use crate::{
    error::{Result, SkillSyncError},
    manifest,
};
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use toml_edit::{value, DocumentMut};

#[derive(Serialize)]
pub struct ManifestEdit {
    pub path: PathBuf,
    pub before: String,
    pub after: String,
}

pub fn preview(path: &Path, source: &str, remove: bool, agents: &[String]) -> Result<ManifestEdit> {
    let normalized = manifest::normalize_source(source);
    let source = normalized.as_str();
    let before = std::fs::read_to_string(path).map_err(SkillSyncError::Io)?;
    let current = manifest::parse_and_validate(&before, path)?;
    // Validate source syntax through the same parser used by installation.
    crate::sources::parse(source)?;
    let mut document = before
        .parse::<DocumentMut>()
        .map_err(|e| SkillSyncError::Config(e.to_string()))?;
    let repos = document["skills"]["repos"]
        .as_array_mut()
        .ok_or_else(|| SkillSyncError::Config("skills.repos must be an array".into()))?;
    let index = current
        .skills
        .repos
        .iter()
        .position(|entry| entry == source);
    if remove {
        if let Some(index) = index {
            repos.remove(index);
        }
    } else if index.is_none() {
        repos.push(source);
    }
    if !agents.is_empty() {
        let mut array = toml_edit::Array::new();
        for agent in agents {
            crate::agents::validate_agent(agent)?;
            array.push(agent);
        }
        document["sync"]["agents"] = value(array);
    }
    let after = document.to_string();
    manifest::parse_and_validate(&after, path)?;
    Ok(ManifestEdit {
        path: path.to_owned(),
        before,
        after,
    })
}

pub fn apply(edit: &ManifestEdit) -> Result<()> {
    if std::fs::read_to_string(&edit.path).map_err(SkillSyncError::Io)? != edit.before {
        return Err(SkillSyncError::Config("manifest changed since preview; retry".into()).into());
    }
    if edit.before == edit.after {
        return Ok(());
    }
    let mut file =
        tempfile::NamedTempFile::new_in(edit.path.parent().unwrap()).map_err(SkillSyncError::Io)?;
    file.as_file()
        .set_permissions(
            std::fs::metadata(&edit.path)
                .map_err(SkillSyncError::Io)?
                .permissions(),
        )
        .map_err(SkillSyncError::Io)?;
    file.write_all(edit.after.as_bytes())
        .map_err(SkillSyncError::Io)?;
    file.flush().map_err(SkillSyncError::Io)?;
    file.persist(&edit.path)
        .map_err(|e| SkillSyncError::Io(e.error))?;
    Ok(())
}
