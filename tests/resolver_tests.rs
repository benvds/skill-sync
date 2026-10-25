use skill_sync::github::{TreeEntry, TreeResponse};
use skill_sync::metadata::SkillMeta;
use skill_sync::resolver::{
    detect_skills, resolve, skill_key, RemoteSkill, RepoSkills, SyncAction,
};
use std::collections::HashMap;

/// Helper: create a TreeResponse from (path, type) pairs.
fn make_tree(entries: &[(&str, &str)]) -> TreeResponse {
    TreeResponse {
        sha: "tree-sha-abc123".to_string(),
        tree: entries
            .iter()
            .map(|(path, entry_type)| TreeEntry {
                path: path.to_string(),
                entry_type: entry_type.to_string(),
                sha: format!("entry-sha-{}", path.replace('/', "-")),
            })
            .collect(),
        truncated: false,
    }
}

// ===== detect_skills tests =====

#[test]
fn detect_root_skill() {
    let tree = make_tree(&[("SKILL.md", "blob"), ("README.md", "blob")]);
    let skills = detect_skills(&tree);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].subdirectory, None);
    assert_eq!(skills[0].tree_sha, "tree-sha-abc123");
}

#[test]
fn detect_root_skill_wins_over_subdirectory() {
    let tree = make_tree(&[
        ("SKILL.md", "blob"),
        ("alpha/SKILL.md", "blob"),
        ("beta/SKILL.md", "blob"),
    ]);
    let skills = detect_skills(&tree);
    assert_eq!(
        skills.len(),
        1,
        "root SKILL.md should win — ignore subdirectories"
    );
    assert_eq!(skills[0].subdirectory, None);
}

#[test]
fn detect_monorepo_skills() {
    let tree = make_tree(&[
        ("README.md", "blob"),
        ("alpha/SKILL.md", "blob"),
        ("beta/SKILL.md", "blob"),
    ]);
    let skills = detect_skills(&tree);
    assert_eq!(skills.len(), 2);

    let subdirs: Vec<Option<String>> = skills.iter().map(|s| s.subdirectory.clone()).collect();
    assert!(subdirs.contains(&Some("alpha".to_string())));
    assert!(subdirs.contains(&Some("beta".to_string())));
}

#[test]
fn detect_no_skills() {
    let tree = make_tree(&[("README.md", "blob"), ("src/main.rs", "blob")]);
    let skills = detect_skills(&tree);
    assert!(skills.is_empty());
}

#[test]
fn detect_finds_deep_nesting() {
    let tree = make_tree(&[("a/b/SKILL.md", "blob"), ("README.md", "blob")]);
    let skills = detect_skills(&tree);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].subdirectory.as_deref(), Some("a/b"));
}

#[test]
fn collection_search_is_recursive_and_stays_inside_the_requested_folder() {
    use skill_sync::resolver::detect_skills_under;
    let tree = make_tree(&[
        ("SKILL.md", "blob"),
        ("skills/team/deep/one/SKILL.md", "blob"),
        ("skills/team/two/SKILL.md", "blob"),
        ("skills/teammate/SKILL.md", "blob"),
        ("elsewhere/SKILL.md", "blob"),
        ("skills/team/fake/SKILL.md", "tree"),
    ]);
    let selected = detect_skills_under(&tree, Some("skills/team"));
    assert_eq!(
        selected
            .iter()
            .map(|s| s.subdirectory.as_deref().unwrap())
            .collect::<Vec<_>>(),
        vec!["skills/team/deep/one", "skills/team/two"]
    );
    assert!(detect_skills_under(&tree, Some("missing")).is_empty());
}

#[test]
fn a_requested_skill_takes_precedence_over_its_descendants() {
    use skill_sync::resolver::detect_skills_under;
    let tree = make_tree(&[
        ("skills/one/SKILL.md", "blob"),
        ("skills/one/examples/another/SKILL.md", "blob"),
    ]);
    let selected = detect_skills_under(&tree, Some("skills/one"));
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].subdirectory.as_deref(), Some("skills/one"));
    // A collection without its own SKILL.md includes every descendant skill.
    assert_eq!(detect_skills_under(&tree, Some("skills")).len(), 2);
}

#[test]
fn detect_ignores_tree_type_skill_md() {
    // A directory named SKILL.md (unlikely but possible)
    let tree = make_tree(&[("SKILL.md", "tree")]);
    let skills = detect_skills(&tree);
    assert!(skills.is_empty(), "tree-type SKILL.md should be ignored");
}

#[test]
fn detect_ignores_commit_type() {
    // Submodule entry
    let tree = make_tree(&[("SKILL.md", "commit")]);
    let skills = detect_skills(&tree);
    assert!(
        skills.is_empty(),
        "commit-type (submodule) should be ignored"
    );
}

#[test]
fn detect_monorepo_sha_from_tree() {
    let tree = make_tree(&[("alpha/SKILL.md", "blob")]);
    let skills = detect_skills(&tree);
    assert_eq!(skills[0].tree_sha, "tree-sha-abc123");
}

// ===== skill_key tests =====

#[test]
fn skill_key_single_repo() {
    assert_eq!(skill_key("owner/repo", &None), "owner/repo");
}

#[test]
fn skill_key_monorepo() {
    assert_eq!(
        skill_key("owner/repo", &Some("alpha".to_string())),
        "owner/repo/alpha"
    );
}

// ===== resolve tests =====

fn make_meta(source: &str, sha: &str, subdirectory: Option<&str>) -> SkillMeta {
    let mut meta = SkillMeta::new(source, sha);
    meta.directory = Some("test-skill".into());
    if let Some(subdir) = subdirectory {
        meta = meta.with_subdirectory(subdir);
    }
    meta
}

#[test]
fn resolve_install_new_skill() {
    let repo_skills = RepoSkills {
        repo: "owner/repo".to_string(),
        skills: vec![RemoteSkill {
            subdirectory: None,
            tree_sha: "tree-sha".to_string(),
        }],
        commit_sha: "commit-abc".to_string(),
    };
    let local: HashMap<String, SkillMeta> = HashMap::new();

    let actions = resolve(&repo_skills, &local);
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0],
        SyncAction::Install {
            repo: "owner/repo".to_string(),
            subdirectory: None,
            remote_sha: "commit-abc".to_string(),
        }
    );
}

#[test]
fn resolve_update_changed_sha() {
    let repo_skills = RepoSkills {
        repo: "owner/repo".to_string(),
        skills: vec![RemoteSkill {
            subdirectory: None,
            tree_sha: "tree-sha".to_string(),
        }],
        commit_sha: "new-commit".to_string(),
    };
    let mut local = HashMap::new();
    local.insert(
        "owner/repo".to_string(),
        make_meta("owner/repo", "old-commit", None),
    );

    let actions = resolve(&repo_skills, &local);
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0],
        SyncAction::Update {
            repo: "owner/repo".to_string(),
            subdirectory: None,
            old_sha: "old-commit".to_string(),
            new_sha: "new-commit".to_string(),
        }
    );
}

#[test]
fn resolve_skip_unchanged() {
    let repo_skills = RepoSkills {
        repo: "owner/repo".to_string(),
        skills: vec![RemoteSkill {
            subdirectory: None,
            tree_sha: "tree-sha".to_string(),
        }],
        commit_sha: "same-sha".to_string(),
    };
    let mut local = HashMap::new();
    local.insert(
        "owner/repo".to_string(),
        make_meta("owner/repo", "same-sha", None),
    );

    let actions = resolve(&repo_skills, &local);
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0],
        SyncAction::Skip {
            repo: "owner/repo".to_string(),
            subdirectory: None,
            sha: "same-sha".to_string(),
        }
    );
}

#[test]
fn resolve_remove_orphan() {
    let repo_skills = RepoSkills {
        repo: "owner/repo".to_string(),
        skills: vec![], // no remote skills
        commit_sha: "commit".to_string(),
    };
    let mut local = HashMap::new();
    local.insert(
        "owner/repo".to_string(),
        make_meta("owner/repo", "old-sha", None),
    );

    let actions = resolve(&repo_skills, &local);
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0],
        SyncAction::Remove {
            repo: "owner/repo".to_string(),
            subdirectory: None,
            local_sha: "old-sha".to_string(),
        }
    );
}

#[test]
fn resolve_multiple_actions() {
    let repo_skills = RepoSkills {
        repo: "owner/repo".to_string(),
        skills: vec![
            RemoteSkill {
                subdirectory: Some("alpha".to_string()),
                tree_sha: "tree".to_string(),
            },
            RemoteSkill {
                subdirectory: Some("beta".to_string()),
                tree_sha: "tree".to_string(),
            },
            RemoteSkill {
                subdirectory: Some("gamma".to_string()),
                tree_sha: "tree".to_string(),
            },
        ],
        commit_sha: "new-commit".to_string(),
    };

    let mut local = HashMap::new();
    // alpha: same sha → Skip
    local.insert(
        "owner/repo/alpha".to_string(),
        make_meta("owner/repo", "new-commit", Some("alpha")),
    );
    // beta: different sha → Update
    local.insert(
        "owner/repo/beta".to_string(),
        make_meta("owner/repo", "old-commit", Some("beta")),
    );
    // gamma: not in local → Install
    // delta: in local but not in remote → Remove
    local.insert(
        "owner/repo/delta".to_string(),
        make_meta("owner/repo", "delta-sha", Some("delta")),
    );

    let actions = resolve(&repo_skills, &local);
    assert_eq!(actions.len(), 4);

    let has_skip = actions
        .iter()
        .any(|a| matches!(a, SyncAction::Skip { subdirectory: Some(s), .. } if s == "alpha"));
    let has_update = actions
        .iter()
        .any(|a| matches!(a, SyncAction::Update { subdirectory: Some(s), .. } if s == "beta"));
    let has_install = actions
        .iter()
        .any(|a| matches!(a, SyncAction::Install { subdirectory: Some(s), .. } if s == "gamma"));
    let has_remove = actions
        .iter()
        .any(|a| matches!(a, SyncAction::Remove { subdirectory: Some(s), .. } if s == "delta"));

    assert!(has_skip, "alpha should be Skip");
    assert!(has_update, "beta should be Update");
    assert!(has_install, "gamma should be Install");
    assert!(has_remove, "delta should be Remove (orphan)");
}

#[test]
fn resolve_idempotent() {
    let repo_skills = RepoSkills {
        repo: "owner/repo".to_string(),
        skills: vec![RemoteSkill {
            subdirectory: None,
            tree_sha: "tree".to_string(),
        }],
        commit_sha: "same-sha".to_string(),
    };
    let mut local = HashMap::new();
    local.insert(
        "owner/repo".to_string(),
        make_meta("owner/repo", "same-sha", None),
    );

    let actions1 = resolve(&repo_skills, &local);
    let actions2 = resolve(&repo_skills, &local);
    assert_eq!(actions1, actions2, "resolve should be idempotent");
    assert!(
        actions1
            .iter()
            .all(|a| matches!(a, SyncAction::Skip { .. })),
        "all actions should be Skip when nothing changed"
    );
}

#[test]
fn resolve_monorepo_skills_tracked_independently() {
    let repo_skills = RepoSkills {
        repo: "owner/repo".to_string(),
        skills: vec![
            RemoteSkill {
                subdirectory: Some("alpha".to_string()),
                tree_sha: "tree".to_string(),
            },
            RemoteSkill {
                subdirectory: Some("beta".to_string()),
                tree_sha: "tree".to_string(),
            },
        ],
        commit_sha: "commit".to_string(),
    };

    let mut local = HashMap::new();
    // Only alpha is installed locally
    local.insert(
        "owner/repo/alpha".to_string(),
        make_meta("owner/repo", "commit", Some("alpha")),
    );

    let actions = resolve(&repo_skills, &local);
    assert_eq!(actions.len(), 2);

    let alpha_action = actions.iter().find(|a| match a {
        SyncAction::Skip {
            subdirectory: Some(s),
            ..
        } => s == "alpha",
        _ => false,
    });
    let beta_action = actions.iter().find(|a| match a {
        SyncAction::Install {
            subdirectory: Some(s),
            ..
        } => s == "beta",
        _ => false,
    });

    assert!(alpha_action.is_some(), "alpha should be Skip (same SHA)");
    assert!(beta_action.is_some(), "beta should be Install (new)");
}

#[test]
fn resolve_remove_only_from_same_repo() {
    let repo_skills = RepoSkills {
        repo: "owner/repo-a".to_string(),
        skills: vec![], // no skills in repo-a
        commit_sha: "commit".to_string(),
    };

    let mut local = HashMap::new();
    // Skill from repo-a → should be removed
    local.insert(
        "owner/repo-a".to_string(),
        make_meta("owner/repo-a", "sha-a", None),
    );
    // Skill from repo-b → should NOT be removed
    local.insert(
        "owner/repo-b".to_string(),
        make_meta("owner/repo-b", "sha-b", None),
    );

    let actions = resolve(&repo_skills, &local);
    assert_eq!(
        actions.len(),
        1,
        "only skills from repo-a should be affected"
    );
    assert!(
        matches!(&actions[0], SyncAction::Remove { repo, .. } if repo == "owner/repo-a"),
        "removed skill should be from repo-a"
    );
}

#[test]
fn legacy_lock_triggers_migration_even_if_sha_is_unchanged() {
    let repo = RepoSkills {
        repo: "owner/repo".into(),
        skills: vec![RemoteSkill {
            subdirectory: None,
            tree_sha: "sha".into(),
        }],
        commit_sha: "sha".into(),
    };
    let legacy = HashMap::from([("owner/repo".into(), SkillMeta::new("owner/repo", "sha"))]);
    assert!(
        matches!(&resolve(&repo, &legacy)[0], SyncAction::Update { old_sha, new_sha, .. } if old_sha == new_sha)
    );
}

#[test]
fn pinned_skills_require_an_existing_skill_file() {
    use skill_sync::resolver::detect_pinned_skill;
    let tree = make_tree(&[
        ("skills/valid/SKILL.md", "blob"),
        ("skills/not-a-file/SKILL.md", "tree"),
    ]);
    assert!(detect_pinned_skill(&tree, "skills/removed").is_none());
    assert!(detect_pinned_skill(&tree, "skills/not-a-file").is_none());
    let skill = detect_pinned_skill(&tree, "skills/valid").unwrap();
    assert_eq!(skill.subdirectory.as_deref(), Some("skills/valid"));
    assert_eq!(skill.tree_sha, tree.sha);
}
