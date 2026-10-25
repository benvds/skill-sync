use skill_sync::metadata::{LockFile, SkillMeta};
use skill_sync::sync::{build_clone_url, execute_remove, skill_dir_name};
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
        skill_dir_name(
            "anthropics/skills",
            &Some("skills/skill-creator".to_string())
        ),
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

    let mut lock_file = LockFile::new();
    lock_file.skills.insert(
        "owner/repo".to_string(),
        SkillMeta::new("owner/repo", "sha1"),
    );
    let lock_path = tmp.path().join(".skills.lock");
    skill_sync::metadata::write_lock_file_atomic(&lock_path, &lock_file).unwrap();

    execute_remove(&skill_dir, &lock_path, &mut lock_file, "owner/repo")
        .expect("execute_remove should succeed");
    assert!(!skill_dir.exists());
}

#[test]
fn execute_remove_nonexistent_returns_error() {
    let tmp = TempDir::new().unwrap();
    let path = std::path::Path::new("/tmp/skill-sync-nonexistent-skill-xyz");
    let lock_path = tmp.path().join(".skills.lock");
    let mut lock_file = LockFile::new();
    skill_sync::metadata::write_lock_file_atomic(&lock_path, &lock_file).unwrap();
    let result = execute_remove(path, &lock_path, &mut lock_file, "owner/repo");
    assert!(
        result.is_err(),
        "execute_remove on nonexistent path should return error"
    );
}

#[test]
fn execute_remove_with_lock_file() {
    let tmp = TempDir::new().unwrap();
    let target_dir = tmp.path();

    // Create a skill directory
    let skill_dir = target_dir.join("owner--repo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "# Skill").unwrap();

    // Create consolidated lock file with entry for this skill
    let mut lock_file = LockFile::new();
    lock_file.skills.insert(
        "owner/repo".to_string(),
        SkillMeta::new("owner/repo", "abc"),
    );
    let lock_path = target_dir.join(".skills.lock");
    skill_sync::metadata::write_lock_file_atomic(&lock_path, &lock_file).unwrap();

    assert!(skill_dir.exists());
    assert!(lock_path.exists());
    assert!(lock_file.skills.contains_key("owner/repo"));

    execute_remove(&skill_dir, &lock_path, &mut lock_file, "owner/repo")
        .expect("execute_remove should succeed");

    assert!(!skill_dir.exists(), "skill directory should be removed");
    // Entry should be removed from in-memory lock_file
    assert!(
        !lock_file.skills.contains_key("owner/repo"),
        "entry should be removed from lock_file"
    );
    // The lock file on disk should also be updated
    let disk_lf = skill_sync::metadata::read_lock_file(&lock_path).unwrap();
    assert!(
        !disk_lf.skills.contains_key("owner/repo"),
        "entry should be removed from disk lock file"
    );
}
