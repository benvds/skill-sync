use skill_sync::manifest::{parse_and_validate, parse_repo_entry};
use std::path::Path;

#[test]
fn test_valid_single_repo() {
    let content = r#"
[skills]
repos = ["owner/repo"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
    let manifest = result.unwrap();
    assert_eq!(manifest.skills.repos.len(), 1);
    assert_eq!(manifest.skills.repos[0], "owner/repo");
}

#[test]
fn test_valid_multiple_repos() {
    let content = r#"
[skills]
repos = ["owner/repo1", "owner/repo2", "other/thing"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
    let manifest = result.unwrap();
    assert_eq!(manifest.skills.repos.len(), 3);
}

#[test]
fn test_invalid_repo_format() {
    let content = r#"
[skills]
repos = ["bad format"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("invalid repo format"));
    assert!(err.contains("bad format"));
}

#[test]
fn test_invalid_repo_no_slash() {
    let content = r#"
[skills]
repos = ["justaname"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
}

#[test]
fn test_duplicate_repos() {
    let content = r#"
[skills]
repos = ["owner/repo", "owner/repo"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("duplicate repo"));
    assert!(err.contains("owner/repo"));
}

#[test]
fn test_whitespace_trimming() {
    let content = r#"
[skills]
repos = [" owner/repo "]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
    let manifest = result.unwrap();
    assert_eq!(manifest.skills.repos[0], "owner/repo");
}

#[test]
fn test_trailing_slash_trimming() {
    let content = r#"
[skills]
repos = ["owner/repo/"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
    let manifest = result.unwrap();
    assert_eq!(manifest.skills.repos[0], "owner/repo");
}

#[test]
fn test_invalid_toml_syntax() {
    let content = "this is not valid {{ toml";
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("invalid TOML syntax"));
}

#[test]
fn test_empty_repos_list() {
    let content = r#"
[skills]
repos = []
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
    let manifest = result.unwrap();
    assert_eq!(manifest.skills.repos.len(), 0);
}

#[test]
fn test_unknown_keys_accepted() {
    let content = r#"
[skills]
repos = ["owner/repo"]

[unknown_section]
foo = "bar"
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
}

#[test]
fn test_repo_pattern_valid_chars() {
    // Test dots, underscores, hyphens in repo names
    let content = r#"
[skills]
repos = ["my-org/my_repo.v2"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
}

#[test]
fn test_repo_pattern_invalid_chars() {
    let content = r#"
[skills]
repos = ["owner/repo with spaces"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
}

#[test]
fn test_missing_skills_section() {
    let content = r#"
[other]
repos = ["owner/repo"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
}

// ─── New tests for monorepo/subdir format ────────────────────────────────────

#[test]
fn test_valid_monorepo_subdir() {
    let content = r#"
[skills]
repos = ["anthropics/skills/skills/skill-creator"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok(), "monorepo subdir entry should be valid: {:?}", result.err());
    let manifest = result.unwrap();
    assert_eq!(manifest.skills.repos[0], "anthropics/skills/skills/skill-creator");
}

#[test]
fn test_valid_deep_subdir() {
    let content = r#"
[skills]
repos = ["owner/repo/a/b/c"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok(), "deep subdir entry should be valid: {:?}", result.err());
}

#[test]
fn test_valid_single_segment_subdir() {
    let content = r#"
[skills]
repos = ["owner/repo/subdir"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok(), "single-segment subdir entry should be valid: {:?}", result.err());
}

#[test]
fn test_owner_repo_still_works() {
    // Backward compatibility: plain owner/repo must still work
    let content = r#"
[skills]
repos = ["owner/repo"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok());
}

#[test]
fn test_single_segment_rejected() {
    let content = r#"
[skills]
repos = ["justaname"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
}

#[test]
fn test_empty_owner_rejected() {
    let content = r#"
[skills]
repos = ["/repo"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err());
}

#[test]
fn test_empty_repo_rejected() {
    // After trimming trailing slash "owner/" becomes "owner" which is invalid
    let content = r#"
[skills]
repos = ["owner/"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err(), "owner/ after trim is 'owner' which should be rejected");
}

#[test]
fn test_trailing_slash_trimmed_subdir() {
    let content = r#"
[skills]
repos = ["owner/repo/subdir/"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok(), "trailing slash on subdir entry should be trimmed: {:?}", result.err());
    let manifest = result.unwrap();
    // After trimming trailing slash, should be "owner/repo/subdir"
    assert_eq!(manifest.skills.repos[0], "owner/repo/subdir");
}

#[test]
fn test_duplicate_different_subdirs() {
    // Same owner/repo but different subdirs are NOT duplicates
    let content = r#"
[skills]
repos = ["owner/repo/sub1", "owner/repo/sub2"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_ok(), "same owner/repo with different subdirs should not be duplicates: {:?}", result.err());
}

#[test]
fn test_duplicate_exact_subdir() {
    // Exact same entry with subdir IS a duplicate
    let content = r#"
[skills]
repos = ["owner/repo/sub", "owner/repo/sub"]
"#;
    let result = parse_and_validate(content, Path::new("test.toml"));
    assert!(result.is_err(), "exact same subdir entry should be a duplicate");
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("duplicate repo"));
}

// ─── parse_repo_entry tests ───────────────────────────────────────────────────

#[test]
fn test_parse_repo_entry_owner_repo() {
    let (owner, repo, subdir) = parse_repo_entry("owner/repo");
    assert_eq!(owner, "owner");
    assert_eq!(repo, "repo");
    assert_eq!(subdir, None);
}

#[test]
fn test_parse_repo_entry_with_subdir() {
    let (owner, repo, subdir) = parse_repo_entry("anthropics/skills/skills/skill-creator");
    assert_eq!(owner, "anthropics");
    assert_eq!(repo, "skills");
    assert_eq!(subdir, Some("skills/skill-creator"));
}

#[test]
fn test_parse_repo_entry_deep_path() {
    let (owner, repo, subdir) = parse_repo_entry("owner/repo/a/b/c");
    assert_eq!(owner, "owner");
    assert_eq!(repo, "repo");
    assert_eq!(subdir, Some("a/b/c"));
}

#[test]
fn test_parse_repo_entry_single_segment_subdir() {
    let (owner, repo, subdir) = parse_repo_entry("owner/repo/subdir");
    assert_eq!(owner, "owner");
    assert_eq!(repo, "repo");
    assert_eq!(subdir, Some("subdir"));
}
