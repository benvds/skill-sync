use skills_sync::metadata::{read_metadata, write_metadata_atomic, SkillMeta};
use tempfile::TempDir;

#[test]
fn test_new_sets_schema_version_1() {
    let meta = SkillMeta::new("owner/repo", "abc123");
    assert_eq!(meta.schema_version, 1);
}

#[test]
fn test_new_sets_source_and_sha() {
    let meta = SkillMeta::new("owner/repo", "abc123def");
    assert_eq!(meta.source, "owner/repo");
    assert_eq!(meta.sha, "abc123def");
}

#[test]
fn test_new_sets_timestamps() {
    let meta = SkillMeta::new("owner/repo", "abc123");
    // Both timestamps should be set and equal (created at same instant)
    assert_eq!(meta.installed_at, meta.updated_at);
}

#[test]
fn test_new_subdirectory_is_none() {
    let meta = SkillMeta::new("owner/repo", "abc123");
    assert_eq!(meta.subdirectory, None);
}

#[test]
fn test_with_subdirectory() {
    let meta = SkillMeta::new("owner/repo", "abc123").with_subdirectory("my-skill");
    assert_eq!(meta.subdirectory, Some("my-skill".to_string()));
}

#[test]
fn test_serialization_contains_required_fields() {
    let meta = SkillMeta::new("owner/repo", "abc123");
    let json = serde_json::to_string_pretty(&meta).unwrap();

    assert!(json.contains("\"schema_version\": 1"));
    assert!(json.contains("\"source\": \"owner/repo\""));
    assert!(json.contains("\"sha\": \"abc123\""));
    assert!(json.contains("\"installed_at\""));
    assert!(json.contains("\"updated_at\""));
}

#[test]
fn test_subdirectory_none_is_omitted() {
    // With skip_serializing_if = "Option::is_none", None subdirectory is omitted
    let meta = SkillMeta::new("owner/repo", "abc123");
    let json = serde_json::to_string_pretty(&meta).unwrap();
    assert!(!json.contains("\"subdirectory\""));
}

#[test]
fn test_subdirectory_some_is_included() {
    let meta = SkillMeta::new("owner/repo", "abc123").with_subdirectory("sub");
    let json = serde_json::to_string_pretty(&meta).unwrap();
    assert!(json.contains("\"subdirectory\": \"sub\""));
}

#[test]
fn test_timestamps_are_iso_8601() {
    let meta = SkillMeta::new("owner/repo", "abc123");
    let json = serde_json::to_string_pretty(&meta).unwrap();
    // ISO 8601 timestamps contain T separator
    // Chrono's serde serialization produces ISO 8601 format
    assert!(json.contains("T"), "Timestamp should contain T separator");
    // Verify the timestamp fields are present and look like ISO 8601
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let installed_at = parsed["installed_at"].as_str().unwrap();
    let updated_at = parsed["updated_at"].as_str().unwrap();
    assert!(installed_at.contains('T'), "installed_at should be ISO 8601");
    assert!(updated_at.contains('T'), "updated_at should be ISO 8601");
    // Should contain UTC indicator (Z or +00:00)
    assert!(
        installed_at.ends_with('Z') || installed_at.contains("+00:00"),
        "Timestamp should indicate UTC: {}",
        installed_at
    );
}

#[test]
fn test_write_and_read_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills-sync-meta.json");

    let meta = SkillMeta::new("owner/repo", "abc123");
    write_metadata_atomic(&path, &meta).unwrap();

    let read_back = read_metadata(&path).unwrap();
    assert_eq!(meta.schema_version, read_back.schema_version);
    assert_eq!(meta.source, read_back.source);
    assert_eq!(meta.sha, read_back.sha);
    assert_eq!(meta.subdirectory, read_back.subdirectory);
    assert_eq!(meta.installed_at, read_back.installed_at);
    assert_eq!(meta.updated_at, read_back.updated_at);
}

#[test]
fn test_write_creates_valid_json_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("meta.json");

    let meta = SkillMeta::new("owner/repo", "sha456");
    write_metadata_atomic(&path, &meta).unwrap();

    assert!(path.exists());
    let content = std::fs::read_to_string(&path).unwrap();
    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["schema_version"], 1);
    assert_eq!(parsed["source"], "owner/repo");
    assert_eq!(parsed["sha"], "sha456");
}

#[test]
fn test_write_with_subdirectory_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("meta.json");

    let meta = SkillMeta::new("owner/monorepo", "def789").with_subdirectory("skill-a");
    write_metadata_atomic(&path, &meta).unwrap();

    let read_back = read_metadata(&path).unwrap();
    assert_eq!(read_back.subdirectory, Some("skill-a".to_string()));
}

#[test]
fn test_write_overwrites_existing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("meta.json");

    let meta1 = SkillMeta::new("owner/repo", "sha1");
    write_metadata_atomic(&path, &meta1).unwrap();

    let meta2 = SkillMeta::new("owner/repo", "sha2");
    write_metadata_atomic(&path, &meta2).unwrap();

    let read_back = read_metadata(&path).unwrap();
    assert_eq!(read_back.sha, "sha2");
}

#[test]
fn test_read_nonexistent_file_returns_error() {
    let result = read_metadata(std::path::Path::new("/nonexistent/path/meta.json"));
    assert!(result.is_err());
}
