//! Versioned stdout contract shared by status and sync. Warnings and interactive
//! prompts belong on stderr, so stdout always contains exactly one JSON value.
use crate::{engine, manifest::Scope, status};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub command: &'static str,
    pub dry_run: bool,
    pub success: bool,
    pub pending_changes: bool,
    pub exit_code: i32,
    pub manifests: Vec<ManifestReport>,
    pub errors: Vec<String>,
    pub created: Option<PathBuf>,
    pub manifest_edits: Vec<crate::edit::ManifestEdit>,
    pub diffs: Vec<crate::diff::SkillDiff>,
    pub source_locks: Vec<PathBuf>,
    pub source_resolutions: Vec<crate::sources::SourceLock>,
}

impl Report {
    pub fn new(command: &'static str, dry_run: bool) -> Self {
        Self {
            schema_version: 1,
            command,
            dry_run,
            success: true,
            pending_changes: false,
            exit_code: 0,
            manifests: Vec::new(),
            errors: Vec::new(),
            created: None,
            manifest_edits: Vec::new(),
            diffs: Vec::new(),
            source_locks: Vec::new(),
            source_resolutions: Vec::new(),
        }
    }
}

#[derive(Serialize)]
pub struct ManifestReport {
    pub manifest: PathBuf,
    pub lock: PathBuf,
    pub scope: Scope,
    pub targets: Vec<PathBuf>,
    pub operations: Vec<Operation>,
    pub unchanged: usize,
    pub conflicts: Vec<String>,
    pub status: Option<status::Snapshot>,
}

#[derive(Serialize)]
pub struct Operation {
    pub action: engine::Action,
    pub skill: String,
    pub target: PathBuf,
    pub outcome: engine::Outcome,
    pub error: Option<String>,
}

impl From<&engine::Operation> for Operation {
    fn from(op: &engine::Operation) -> Self {
        Self {
            action: op.action,
            skill: op.skill.clone(),
            target: op.target.clone(),
            outcome: engine::Outcome::Planned,
            error: None,
        }
    }
}
