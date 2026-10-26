use crate::error::SkillSyncError;
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

static REPO_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+(/[a-zA-Z0-9._-]+)*$").unwrap());

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub skills: Skills,
    pub sync: Option<SyncConfig>,
    #[serde(default)]
    pub agents: BTreeMap<String, AgentPaths>,
}

#[derive(Debug, Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    Project,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SyncConfig {
    pub scope: Option<Scope>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub mode: CopyMode,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CopyMode {
    #[default]
    Copy,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AgentPaths {
    pub user_dir: Option<String>,
    pub project_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Skills {
    pub repos: Vec<String>,
}

/// Split an optional Git tag, branch or commit from a source, after its path.
pub fn split_revision(entry: &str) -> (&str, Option<&str>) {
    if entry.starts_with("local:") {
        return (entry, None);
    }
    if entry.starts_with("git+") {
        return entry
            .rsplit_once('#')
            .map(|(s, r)| (s, Some(r)))
            .unwrap_or((entry, None));
    }
    match entry.split_once('@') {
        Some((source, revision)) => (source, Some(revision)),
        None => (entry, None),
    }
}

pub fn normalize_source(entry: &str) -> String {
    let (source, revision) = split_revision(entry.trim());
    if source.starts_with("local:") {
        return source.to_owned();
    }
    let source = source.trim_end_matches('/');
    match revision {
        Some(revision) if source.starts_with("git+") => format!("{source}#{revision}"),
        Some(revision) => format!("{source}@{revision}"),
        None => source.to_owned(),
    }
}

pub(crate) fn valid_revision(revision: &str) -> bool {
    !revision.is_empty()
        && !revision.starts_with('-')
        && !revision.contains("..")
        && !revision.contains("@{")
        && revision.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && !part.ends_with('.')
                && !part.ends_with(".lock")
        })
        && revision
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/+".contains(c))
}

pub(crate) fn valid_github_source(source: &str) -> bool {
    REPO_PATTERN.is_match(source) && !source.split('/').any(|part| part == "." || part == "..")
}

/// Parse a manifest repo entry into (owner, repo_name, optional subdirectory).
///
/// Format: `owner/repo` or `owner/repo/path/to/skill`, optionally with `@revision`.
/// The first two path segments are always owner and repo.
/// Any remaining segments form the subdirectory path.
///
/// Examples:
///   "anthropics/skills"                      → ("anthropics", "skills", None)
///   "anthropics/skills/skills/skill-creator" → ("anthropics", "skills", Some("skills/skill-creator"))
///   "owner/repo/a/b/c"                       → ("owner", "repo", Some("a/b/c"))
pub fn parse_repo_entry(entry: &str) -> (&str, &str, Option<&str>) {
    let (source, _) = split_revision(entry);
    let mut parts = source.splitn(3, '/');
    let owner = parts.next().unwrap_or("");
    let repo = parts.next().unwrap_or("");
    let subdir = parts.next(); // None if only owner/repo
    (owner, repo, subdir)
}

/// Parse and validate a .skills.toml manifest.
///
/// Validates: TOML syntax, repo format (owner/repo or owner/repo/subdir pattern), duplicates.
/// Auto-trims whitespace and trailing slashes.
/// Warns on unknown keys and empty repos list.
pub fn parse_and_validate(content: &str, path: &Path) -> crate::error::Result<Manifest> {
    // Step 1: Parse as generic table to detect unknown keys
    let table: toml::Table = toml::from_str(content).map_err(|e| {
        let span = e
            .span()
            .map(|s| miette::SourceSpan::new(s.start.into(), (s.end - s.start).max(1)));
        SkillSyncError::InvalidToml {
            src: miette::NamedSource::new(path.display().to_string(), content.to_string()),
            span: span.unwrap_or_else(|| miette::SourceSpan::new(0.into(), 1_usize)),
        }
    })?;

    // Warn about unknown top-level keys
    let known_keys = ["skills", "sync", "agents"];
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
        let span = e
            .span()
            .map(|s| miette::SourceSpan::new(s.start.into(), (s.end - s.start).max(1)));
        SkillSyncError::InvalidToml {
            src: miette::NamedSource::new(path.display().to_string(), content.to_string()),
            span: span.unwrap_or_else(|| miette::SourceSpan::new(0.into(), 1_usize)),
        }
    })?;

    // Validate agent names even for empty manifests (which may remove installations).
    if let Some(sync) = &manifest.sync {
        for agent in &sync.agents {
            crate::agents::validate_agent(agent)?;
        }
    }
    for agent in manifest.agents.keys() {
        crate::agents::validate_agent(agent)?;
    }

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
        .map(|r| normalize_source(&r))
        .collect();

    // Step 5: Validate repo format
    for repo in &manifest.skills.repos {
        crate::sources::parse(repo)
            .map_err(|_| SkillSyncError::InvalidRepo { repo: repo.clone() })?;
    }

    // Step 6: Check for duplicates
    let mut seen = HashSet::new();
    for repo in &manifest.skills.repos {
        if !seen.insert(repo.clone()) {
            return Err(SkillSyncError::DuplicateRepo { repo: repo.clone() }.into());
        }
    }

    Ok(manifest)
}
