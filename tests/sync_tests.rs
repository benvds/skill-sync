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

#[test]
fn skill_dir_name_multi_segment_subdir() {
    assert_eq!(
        skill_dir_name("anthropics/skills", &Some("skills/skill-creator".to_string())),
        "anthropics--skills--skills--skill-creator"
    );
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

// --- execute_remove tests ---

#[test]
fn execute_remove_deletes_directory() {
    let tmp = TempDir::new().unwrap();
    // Create a skill directory manually
    let skill_dir = tmp.path().join("owner--repo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("README.md"), "hello").unwrap();

    assert!(skill_dir.exists());
    execute_remove(&skill_dir).expect("execute_remove should succeed");
    assert!(!skill_dir.exists());
}

#[test]
fn execute_remove_nonexistent_returns_error() {
    let path = std::path::Path::new("/tmp/skills-sync-nonexistent-skill-xyz");
    let result = execute_remove(path);
    assert!(result.is_err(), "execute_remove on nonexistent path should return error");
}
