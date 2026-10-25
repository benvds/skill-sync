use skill_sync::init::run_init;
use tempfile::TempDir;

#[test]
fn test_init_creates_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    let result = run_init(&path);
    assert!(result.is_ok());
    assert!(path.exists());
}

#[test]
fn test_init_content_has_skills_section() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    run_init(&path).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    assert!(content.contains("[skills]"), "Should contain [skills] section");
    assert!(
        content.contains("repos = ["),
        "Should contain repos array"
    );
}

#[test]
fn test_init_content_has_example_repos() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    run_init(&path).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    assert!(
        content.contains("anthropics/claude-skills"),
        "Should contain example repo"
    );
    assert!(
        content.contains("example-org/skill-collection"),
        "Should contain monorepo example"
    );
}

#[test]
fn test_init_content_has_usage_docs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    run_init(&path).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    assert!(
        content.contains("Usage"),
        "Should contain usage documentation"
    );
    assert!(
        content.contains("Format"),
        "Should contain format documentation"
    );
    assert!(
        content.contains("Examples"),
        "Should contain examples section"
    );
}

#[test]
fn test_init_content_has_header() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    run_init(&path).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    assert!(
        content.contains("skill-sync manifest"),
        "Should contain header"
    );
}

#[test]
fn test_init_refuses_overwrite() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    // Create the file first
    std::fs::write(&path, "existing content").unwrap();

    let result = run_init(&path);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("already exists"),
        "Error should mention file already exists: {}",
        err
    );

    // Verify original content is preserved
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "existing content");
}

#[test]
fn test_init_content_is_valid_toml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    run_init(&path).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Should parse as valid TOML
    let parsed: toml::Table = toml::from_str(&content).unwrap();
    assert!(parsed.contains_key("skills"), "Parsed TOML should have skills key");
}

#[test]
fn test_init_generated_manifest_passes_validation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.toml");

    run_init(&path).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();

    // Should pass our own manifest validation
    let result = skill_sync::manifest::parse_and_validate(&content, &path);
    assert!(
        result.is_ok(),
        "Generated manifest should pass validation: {:?}",
        result.err()
    );
}
