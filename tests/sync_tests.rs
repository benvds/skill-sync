use skills_sync::metadata::meta_path_for_skill;
use skills_sync::sync::{build_clone_url, execute_remove, skill_dir_name};
use tempfile::TempDir;

// --- skill_dir_name tests ---

#[test]
fn skill_dir_name_single() {
    assert_eq!(skill_dir_name("owner/repo", &None), "owner--repo");
}

#[test]
fn skill_dir_name_monorepo() {
    assert_eq!(
        skill_dir_name("owner/repo", &Some("subdir".to_string())),
        "owner--repo--subdir"
    );
}

#[test]
fn skill_dir_name_special_chars() {
    assert_eq!(skill_dir_name("my.org/my-repo", &None), "my.org--my-repo");
}

// --- build_clone_url tests ---

#[test]
fn build_clone_url_without_token() {
    assert_eq!(
        build_clone_url("owner/repo", None),
        "https://github.com/owner/repo.git"
    );
}

#[test]
fn build_clone_url_with_token() {
    assert_eq!(
        build_clone_url("owner/repo", Some("ghp_abc123")),
        "https://ghp_abc123@github.com/owner/repo.git"
    );
}

// --- meta_path_for_skill tests ---

#[test]
fn meta_path_single() {
    let target_dir = std::path::Path::new("/tmp/skills");
    let result = meta_path_for_skill(target_dir, "owner--repo");
    assert_eq!(result, std::path::PathBuf::from("/tmp/skills/owner--repo.lock"));
}

#[test]
fn meta_path_monorepo() {
    let target_dir = std::path::Path::new("/tmp/skills");
    let result = meta_path_for_skill(target_dir, "owner--repo--subdir");
    assert_eq!(result, std::path::PathBuf::from("/tmp/skills/owner--repo--subdir.lock"));
}

// --- execute_remove tests ---

#[test]
fn execute_remove_deletes_directory() {
    let tmp = TempDir::new().unwrap();
    // Create a skill directory manually
    let skill_dir = tmp.path().join("owner--repo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("README.md"), "hello").unwrap();

    assert!(skill_dir.exists());
    execute_remove(&skill_dir, None).expect("execute_remove should succeed");
    assert!(!skill_dir.exists());
}

#[test]
fn execute_remove_nonexistent_returns_error() {
    let path = std::path::Path::new("/tmp/skills-sync-nonexistent-skill-xyz");
    let result = execute_remove(path, None);
    assert!(result.is_err(), "execute_remove on nonexistent path should return error");
}

#[test]
fn execute_remove_with_lock_file() {
    let tmp = TempDir::new().unwrap();
    let target_dir = tmp.path();

    // Create a skill directory
    let skill_dir = target_dir.join("owner--repo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# Skill").unwrap();

    // Create the sibling lock file
    let lock_path = target_dir.join("owner--repo.lock");
    std::fs::write(&lock_path, "schema_version = 1\nsource = \"owner/repo\"\nsha = \"abc\"\n").unwrap();

    assert!(skill_dir.exists());
    assert!(lock_path.exists());

    execute_remove(&skill_dir, Some(&lock_path)).expect("execute_remove should succeed");

    assert!(!skill_dir.exists(), "skill directory should be removed");
    assert!(!lock_path.exists(), "sibling lock file should be removed");
}
