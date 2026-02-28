use skills_sync::config::discover_manifest_in;
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
