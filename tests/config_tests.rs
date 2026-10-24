use skills_sync::config::discover_manifest_in;
use skills_sync::config::{is_project_dir, target_dir_for_manifest};
use tempfile::TempDir;

#[test]
fn test_explicit_config_exists() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("custom.toml");
    std::fs::write(&config_path, "[skills]\nrepos = []\n").unwrap();

    let result = discover_manifest_in(dir.path(), Some(&config_path));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), config_path);
}

#[test]
fn test_explicit_config_missing() {
    let dir = TempDir::new().unwrap();
    let missing_path = dir.path().join("nonexistent.toml");

    let result = discover_manifest_in(dir.path(), Some(&missing_path));
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("manifest not found"));
}

#[test]
fn test_local_manifest_discovery() {
    let dir = TempDir::new().unwrap();
    let manifest_path = dir.path().join(".skills.toml");
    std::fs::write(&manifest_path, "[skills]\nrepos = []\n").unwrap();

    let result = discover_manifest_in(dir.path(), None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), manifest_path);
}

#[test]
fn test_no_manifest_anywhere() {
    let dir = TempDir::new().unwrap();

    // No .skills.toml in the temp dir, and we don't have a global one
    let result = discover_manifest_in(dir.path(), None);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("manifest not found"));
}

#[test]
fn test_explicit_config_takes_priority_over_local() {
    let dir = TempDir::new().unwrap();

    // Create both local and explicit config
    let local_path = dir.path().join(".skills.toml");
    std::fs::write(&local_path, "[skills]\nrepos = []\n").unwrap();

    let explicit_path = dir.path().join("custom.toml");
    std::fs::write(&explicit_path, "[skills]\nrepos = [\"a/b\"]\n").unwrap();

    let result = discover_manifest_in(dir.path(), Some(&explicit_path));
    assert!(result.is_ok());
    // Should return the explicit path, not the local one
    assert_eq!(result.unwrap(), explicit_path);
}

#[test]
fn test_error_includes_init_hint() {
    let dir = TempDir::new().unwrap();
    let result = discover_manifest_in(dir.path(), None);
    assert!(result.is_err());
    let err = format!("{:?}", result.unwrap_err());
    assert!(
        err.contains("--init"),
        "Error should suggest --init: {}",
        err
    );
}

#[test]
fn plain_manifest_is_discovered_and_hidden_manifest_takes_priority() {
    let dir = TempDir::new().unwrap();
    let plain = dir.path().join("skills.toml");
    std::fs::write(&plain, "[skills]\nrepos = []\n").unwrap();
    assert_eq!(discover_manifest_in(dir.path(), None).unwrap(), plain);
    let hidden = dir.path().join(".skills.toml");
    std::fs::write(&hidden, "[skills]\nrepos = []\n").unwrap();
    assert_eq!(discover_manifest_in(dir.path(), None).unwrap(), hidden);
}

#[test]
fn target_layouts_are_relative_to_manifest() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    for (folder, marker, expected) in [
        ("project", Some("Cargo.toml"), "project/.agents/skills"),
        ("worktree", Some(".git"), "worktree/.agents/skills"),
        (
            "new-project",
            Some("AGENTS.md"),
            "new-project/.agents/skills",
        ),
        ("standalone", None, "standalone"),
        ("skills", Some(".git"), "skills"),
        (".agents", None, ".agents/skills"),
        (".codex", None, ".codex/skills"),
        (".claude", None, ".claude/skills"),
        (
            "home/.agents/skills",
            Some("package.json"),
            "home/.agents/skills",
        ),
        ("home/.codex/skills", None, "home/.codex/skills"),
        ("project/.agents/skills", None, "project/.agents/skills"),
    ] {
        let base = root.join(folder);
        std::fs::create_dir_all(&base).unwrap();
        if let Some(marker) = marker {
            std::fs::write(base.join(marker), "").unwrap();
        }
        assert_eq!(
            target_dir_for_manifest(&base.join("skills.toml")),
            root.join(expected),
            "{folder}"
        );
    }
}

#[test]
fn project_detection_handles_git_worktree_files_and_agent_folders() {
    let dir = TempDir::new().unwrap();
    assert!(!is_project_dir(dir.path()));
    std::fs::write(dir.path().join(".git"), "gitdir: elsewhere").unwrap();
    assert!(is_project_dir(dir.path()));
    std::fs::remove_file(dir.path().join(".git")).unwrap();
    std::fs::create_dir(dir.path().join(".agents")).unwrap();
    assert!(is_project_dir(dir.path()));
}

#[cfg(unix)]
#[test]
fn symlinked_skills_directory_is_not_nested() {
    let dir = TempDir::new().unwrap();
    let collection = dir.path().join("skills");
    std::fs::create_dir(&collection).unwrap();
    let link = dir.path().join("shared");
    std::os::unix::fs::symlink(&collection, &link).unwrap();
    assert_eq!(
        target_dir_for_manifest(&link.join(".skills.toml")),
        collection.canonicalize().unwrap()
    );

    let versioned = dir.path().join("shared-collection");
    std::fs::create_dir(&versioned).unwrap();
    std::fs::write(versioned.join(".git"), "gitdir: elsewhere").unwrap();
    let link = dir.path().join(".agents/skills");
    std::fs::create_dir(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&versioned, &link).unwrap();
    assert_eq!(
        target_dir_for_manifest(&link.join("skills.toml")),
        versioned.canonicalize().unwrap()
    );
}
