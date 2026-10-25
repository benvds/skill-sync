use skill_sync::github::{
    AuthMethod, GitHubClient, RepoInfo, TreeEntry, TreeResponse,
};
use std::sync::Mutex;

// Mutex to serialize env var tests (env vars are process-global)
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn discover_token_from_env() {
    let _lock = ENV_MUTEX.lock().unwrap();
    // Save and restore original value
    let original = std::env::var("GITHUB_TOKEN").ok();
    std::env::set_var("GITHUB_TOKEN", "test-token-123");
    let (token, method) = skill_sync::github::discover_token();
    // Restore
    match original {
        Some(v) => std::env::set_var("GITHUB_TOKEN", v),
        None => std::env::remove_var("GITHUB_TOKEN"),
    }
    assert_eq!(token, Some("test-token-123".to_string()));
    assert_eq!(method, AuthMethod::EnvVar);
}

#[test]
fn discover_token_empty_env_falls_through() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let original = std::env::var("GITHUB_TOKEN").ok();
    std::env::set_var("GITHUB_TOKEN", "");
    let (_token, method) = skill_sync::github::discover_token();
    // Restore
    match original {
        Some(v) => std::env::set_var("GITHUB_TOKEN", v),
        None => std::env::remove_var("GITHUB_TOKEN"),
    }
    // Empty GITHUB_TOKEN should fall through — method is either GhCli or Unauthenticated
    assert_ne!(method, AuthMethod::EnvVar);
}

#[test]
fn tree_response_deserialize() {
    let json = r#"{
        "sha": "abc123",
        "tree": [
            {
                "path": "SKILL.md",
                "type": "blob",
                "sha": "def456"
            },
            {
                "path": "src",
                "type": "tree",
                "sha": "ghi789"
            }
        ],
        "truncated": false
    }"#;

    let response: TreeResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.sha, "abc123");
    assert_eq!(response.tree.len(), 2);
    assert!(!response.truncated);
}

#[test]
fn tree_entry_type_rename() {
    let json = r#"{"path": "README.md", "type": "blob", "sha": "abc"}"#;
    let entry: TreeEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.path, "README.md");
    assert_eq!(entry.entry_type, "blob");
    assert_eq!(entry.sha, "abc");
}

#[test]
fn tree_entry_tree_type() {
    let json = r#"{"path": "src", "type": "tree", "sha": "def"}"#;
    let entry: TreeEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.entry_type, "tree");
}

#[test]
fn repo_info_deserialize() {
    let json = r#"{"default_branch": "main"}"#;
    let info: RepoInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.default_branch, "main");
}

#[test]
fn repo_info_deserialize_master() {
    let json = r#"{"default_branch": "master"}"#;
    let info: RepoInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.default_branch, "master");
}

#[test]
fn repo_info_ignores_extra_fields() {
    // GitHub API returns many fields — we only need default_branch
    let json = r#"{
        "id": 123,
        "name": "test-repo",
        "full_name": "owner/test-repo",
        "private": false,
        "default_branch": "develop"
    }"#;
    let info: RepoInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.default_branch, "develop");
}

#[test]
fn client_new_succeeds() {
    let client = GitHubClient::new(None, AuthMethod::Unauthenticated);
    assert!(client.is_ok());
}

#[test]
fn client_new_with_token() {
    let client = GitHubClient::new(Some("token".to_string()), AuthMethod::EnvVar);
    assert!(client.is_ok());
    let client = client.unwrap();
    assert_eq!(client.auth_method, AuthMethod::EnvVar);
}
