use crate::github::TreeResponse;
use crate::metadata::SkillMeta;
use std::collections::HashMap;

/// A skill detected in a remote repository.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteSkill {
    /// None for single-skill repos (SKILL.md at root).
    /// Some("subdir") for monorepo skills.
    pub subdirectory: Option<String>,
    /// The tree SHA from the parent TreeResponse.
    pub tree_sha: String,
}

/// Groups a repo's remote skills with its HEAD commit SHA.
#[derive(Debug, Clone)]
pub struct RepoSkills {
    /// The repo identifier (owner/repo).
    pub repo: String,
    /// Skills detected in this repo.
    pub skills: Vec<RemoteSkill>,
    /// The HEAD commit SHA for change detection.
    pub commit_sha: String,
}

/// An action the sync engine should take for a skill.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncAction {
    /// New skill to install (not present locally).
    Install {
        repo: String,
        subdirectory: Option<String>,
        remote_sha: String,
    },
    /// Existing skill with changed SHA — needs update.
    Update {
        repo: String,
        subdirectory: Option<String>,
        old_sha: String,
        new_sha: String,
    },
    /// Orphaned skill — present locally but not in remote.
    Remove {
        repo: String,
        subdirectory: Option<String>,
        local_sha: String,
    },
    /// Unchanged skill — no action needed.
    Skip {
        repo: String,
        subdirectory: Option<String>,
        sha: String,
    },
}

/// Build a lookup key for matching remote skills against local metadata.
///
/// For single-skill repos: "owner/repo"
/// For monorepo skills: "owner/repo/subdir"
pub fn skill_key(repo: &str, subdirectory: &Option<String>) -> String {
    match subdirectory {
        Some(subdir) => format!("{}/{}", repo, subdir),
        None => repo.to_string(),
    }
}

/// Detect skills in a repository from its tree data.
///
/// Rules:
/// 1. Root SKILL.md wins — if present, treat as single-skill repo
/// 2. One-level-deep subdirectory SKILL.md files detected for monorepos
/// 3. Only blob type entries are considered (not tree or commit)
/// 4. Deeply nested SKILL.md files (2+ levels) are ignored
pub fn detect_skills(tree: &TreeResponse) -> Vec<RemoteSkill> {
    // Check root first — root SKILL.md wins
    let has_root_skill = tree
        .tree
        .iter()
        .any(|e| e.path == "SKILL.md" && e.entry_type == "blob");

    if has_root_skill {
        return vec![RemoteSkill {
            subdirectory: None,
            tree_sha: tree.sha.clone(),
        }];
    }

    // Check one-level-deep subdirectories
    tree.tree
        .iter()
        .filter(|e| {
            e.entry_type == "blob"
                && e.path.ends_with("/SKILL.md")
                && e.path.matches('/').count() == 1
        })
        .map(|e| {
            let subdir = e.path.strip_suffix("/SKILL.md").unwrap().to_string();
            RemoteSkill {
                subdirectory: Some(subdir),
                tree_sha: tree.sha.clone(),
            }
        })
        .collect()
}

/// A pinned subdirectory must actually contain a SKILL.md file in the remote tree.
pub fn detect_pinned_skill(tree: &TreeResponse, subdirectory: &str) -> Option<RemoteSkill> {
    let skill_file = format!("{subdirectory}/SKILL.md");
    tree.tree
        .iter()
        .any(|entry| entry.entry_type == "blob" && entry.path == skill_file)
        .then(|| RemoteSkill {
            subdirectory: Some(subdirectory.into()),
            tree_sha: tree.sha.clone(),
        })
}

/// Resolve sync actions by comparing remote skills against local metadata.
///
/// Pure function — no I/O. Takes remote state and local state, returns actions.
///
/// Actions:
/// - Install: remote skill not in local metadata
/// - Update: remote skill's commit SHA differs from local metadata SHA
/// - Skip: remote skill's commit SHA matches local metadata SHA
/// - Remove: local skill (same repo) not in remote skills (orphan)
pub fn resolve(
    repo_skills: &RepoSkills,
    local_metadata: &HashMap<String, SkillMeta>,
) -> Vec<SyncAction> {
    let mut actions = Vec::new();
    let repo = &repo_skills.repo;
    let mut matched_keys = Vec::new();

    // Install/Update/Skip for each remote skill
    for skill in &repo_skills.skills {
        let key = skill_key(repo, &skill.subdirectory);
        matched_keys.push(key.clone());

        match local_metadata.get(&key) {
            None => {
                actions.push(SyncAction::Install {
                    repo: repo.clone(),
                    subdirectory: skill.subdirectory.clone(),
                    remote_sha: repo_skills.commit_sha.clone(),
                });
            }
            // Reinstall legacy owner--repo directories under their portable skill name.
            Some(meta) if meta.sha != repo_skills.commit_sha || meta.directory.is_none() => {
                actions.push(SyncAction::Update {
                    repo: repo.clone(),
                    subdirectory: skill.subdirectory.clone(),
                    old_sha: meta.sha.clone(),
                    new_sha: repo_skills.commit_sha.clone(),
                });
            }
            Some(_) => {
                actions.push(SyncAction::Skip {
                    repo: repo.clone(),
                    subdirectory: skill.subdirectory.clone(),
                    sha: repo_skills.commit_sha.clone(),
                });
            }
        }
    }

    // Remove orphans: local skills from this repo not in remote
    for (key, meta) in local_metadata {
        if meta.source == *repo && !matched_keys.contains(key) {
            actions.push(SyncAction::Remove {
                repo: repo.clone(),
                subdirectory: meta.subdirectory.clone(),
                local_sha: meta.sha.clone(),
            });
        }
    }

    actions
}
