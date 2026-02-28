use crate::error::SkillsSyncError;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

static REPO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+$").unwrap()
});

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub skills: Skills,
}

#[derive(Debug, Deserialize)]
pub struct Skills {
    pub repos: Vec<String>,
}

/// Parse and validate a .skills.toml manifest.
///
/// Validates: TOML syntax, repo format (owner/repo pattern), duplicates.
/// Auto-trims whitespace and trailing slashes.
/// Warns on unknown keys and empty repos list.
pub fn parse_and_validate(content: &str, path: &Path) -> crate::error::Result<Manifest> {
    // Step 1: Parse as generic table to detect unknown keys
    let table: toml::Table = toml::from_str(content).map_err(|e| {
        let span = e.span().map(|s| {
            miette::SourceSpan::new(s.start.into(), (s.end - s.start).max(1).into())
        });
        SkillsSyncError::InvalidToml {
            src: miette::NamedSource::new(path.display().to_string(), content.to_string()),
            span: span.unwrap_or_else(|| miette::SourceSpan::new(0.into(), 1_usize.into())),
        }
    })?;

    // Warn about unknown top-level keys
    let known_keys = ["skills"];
    for key in table.keys() {
        if !known_keys.contains(&key.as_str()) {
            eprintln!(
                "warning: unknown key `{}` in manifest (ignored, forward-compatible)",
                key
            );
        }
    }

    // Step 2: Deserialize into typed struct
    let mut manifest: Manifest = toml::from_str(content).map_err(|e| {
        let span = e.span().map(|s| {
            miette::SourceSpan::new(s.start.into(), (s.end - s.start).max(1).into())
        });
        SkillsSyncError::InvalidToml {
            src: miette::NamedSource::new(path.display().to_string(), content.to_string()),
            span: span.unwrap_or_else(|| miette::SourceSpan::new(0.into(), 1_usize.into())),
        }
    })?;

    // Step 3: Warn on empty repos
    if manifest.skills.repos.is_empty() {
        eprintln!("warning: No repos listed in manifest. Add repos to [skills].repos.");
        return Ok(manifest);
    }

    // Step 4: Trim whitespace and trailing slashes
    manifest.skills.repos = manifest
        .skills
        .repos
        .into_iter()
        .map(|r| r.trim().trim_end_matches('/').to_string())
        .collect();

    // Step 5: Validate repo format
    for repo in &manifest.skills.repos {
        if !REPO_PATTERN.is_match(repo) {
            return Err(SkillsSyncError::InvalidRepo {
                repo: repo.clone(),
            }
            .into());
        }
    }

    // Step 6: Check for duplicates
    let mut seen = HashSet::new();
    for repo in &manifest.skills.repos {
        if !seen.insert(repo.clone()) {
            return Err(SkillsSyncError::DuplicateRepo {
                repo: repo.clone(),
            }
            .into());
        }
    }

    Ok(manifest)
}
