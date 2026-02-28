use skills_sync::manifest::parse_and_validate;
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
