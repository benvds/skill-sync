use skills_sync::sync::{build_clone_url, skill_dir_name};

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
