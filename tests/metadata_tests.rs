use skill_sync::metadata::{read_lock_file, write_lock_file_atomic, LockFile, SkillMeta};
use tempfile::TempDir;

// --- LockFile tests ---

#[test]
fn test_lock_file_new_creates_empty() {
    let lf = LockFile::new();
    assert_eq!(lf.schema_version, 1);
    assert!(lf.skills.is_empty());
}

#[test]
fn test_lock_file_serializes_multiple_entries() {
    let mut lf = LockFile::new();
    let meta1 = SkillMeta::new("owner/repo", "sha111");
    let meta2 = SkillMeta::new("other/pkg", "sha222").with_subdirectory("skills/tool");
    lf.skills.insert("owner/repo".to_string(), meta1);
    lf.skills.insert("other/pkg/skills/tool".to_string(), meta2);

    let toml_str = toml::to_string_pretty(&lf).unwrap();
    assert!(toml_str.contains("schema_version = 1"));
    assert!(toml_str.contains("[skills.\"owner/repo\"]"));
    assert!(toml_str.contains("[skills.\"other/pkg/skills/tool\"]"));
}

#[test]
fn test_lock_file_roundtrip_preserves_all_fields() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.lock");

    let mut lf = LockFile::new();
    let meta1 = SkillMeta::new("owner/repo", "abc123");
    let meta2 = SkillMeta::new("other/monorepo", "def456").with_subdirectory("skill-a");
    lf.skills.insert("owner/repo".to_string(), meta1.clone());
    lf.skills
        .insert("other/monorepo/skill-a".to_string(), meta2.clone());

    write_lock_file_atomic(&path, &lf).unwrap();
    let read_back = read_lock_file(&path).unwrap();

    assert_eq!(read_back.schema_version, 1);
    assert_eq!(read_back.skills.len(), 2);

    let r1 = read_back.skills.get("owner/repo").unwrap();
    assert_eq!(r1.source, "owner/repo");
    assert_eq!(r1.sha, "abc123");
    assert_eq!(r1.subdirectory, None);

    let r2 = read_back.skills.get("other/monorepo/skill-a").unwrap();
    assert_eq!(r2.source, "other/monorepo");
    assert_eq!(r2.sha, "def456");
    assert_eq!(r2.subdirectory, Some("skill-a".to_string()));
}

#[test]
fn test_write_lock_file_atomic_creates_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.lock");

    assert!(!path.exists());
    let lf = LockFile::new();
    write_lock_file_atomic(&path, &lf).unwrap();
    assert!(path.exists());
}

#[test]
fn test_read_lock_file_nonexistent_returns_empty() {
    let result = read_lock_file(std::path::Path::new("/nonexistent/path/.skills.lock"));
    assert!(result.is_ok());
    let lf = result.unwrap();
    assert!(lf.skills.is_empty());
    assert_eq!(lf.schema_version, 1);
}

#[test]
fn test_lock_file_insert_and_remove() {
    let mut lf = LockFile::new();
    lf.skills.insert(
        "owner/repo".to_string(),
        SkillMeta::new("owner/repo", "sha1"),
    );
    assert_eq!(lf.skills.len(), 1);
    lf.skills.remove("owner/repo");
    assert!(lf.skills.is_empty());
}

#[test]
fn test_lock_file_skill_meta_omits_subdirectory_when_none() {
    let mut lf = LockFile::new();
    lf.skills.insert(
        "owner/repo".to_string(),
        SkillMeta::new("owner/repo", "sha1"),
    );
    let toml_str = toml::to_string_pretty(&lf).unwrap();
    // subdirectory should not appear for entries with None subdirectory
    // The skills section may have "subdirectory" but it's for entries that have it set
    let skills_section_start = toml_str.find("[skills.\"owner/repo\"]").unwrap();
    let skills_section = &toml_str[skills_section_start..];
    assert!(!skills_section.contains("subdirectory"));
}

// --- SkillMeta tests (updated, no schema_version on SkillMeta) ---

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
fn test_skill_meta_serialization_contains_required_fields() {
    let mut lf = LockFile::new();
    lf.skills.insert(
        "owner/repo".to_string(),
        SkillMeta::new("owner/repo", "abc123"),
    );
    let toml_str = toml::to_string_pretty(&lf).unwrap();
    assert!(toml_str.contains("source = \"owner/repo\""));
    assert!(toml_str.contains("sha = \"abc123\""));
    assert!(toml_str.contains("installed_at"));
    assert!(toml_str.contains("updated_at"));
}

#[test]
fn test_subdirectory_none_is_omitted() {
    // With skip_serializing_if = "Option::is_none", None subdirectory is omitted
    let meta = SkillMeta::new("owner/repo", "abc123");
    let toml_str = toml::to_string_pretty(&meta).unwrap();
    assert!(!toml_str.contains("subdirectory"));
}

#[test]
fn test_subdirectory_some_is_included() {
    let meta = SkillMeta::new("owner/repo", "abc123").with_subdirectory("sub");
    let toml_str = toml::to_string_pretty(&meta).unwrap();
    assert!(toml_str.contains("subdirectory = \"sub\""));
}

#[test]
fn test_timestamps_are_iso_8601() {
    let meta = SkillMeta::new("owner/repo", "abc123");
    let toml_str = toml::to_string_pretty(&meta).unwrap();
    // ISO 8601 timestamps contain T separator
    assert!(
        toml_str.contains("T"),
        "Timestamp should contain T separator"
    );
}

#[test]
fn test_write_with_subdirectory_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.lock");

    let mut lf = LockFile::new();
    lf.skills.insert(
        "owner/monorepo/skill-a".to_string(),
        SkillMeta::new("owner/monorepo", "def789").with_subdirectory("skill-a"),
    );

    write_lock_file_atomic(&path, &lf).unwrap();
    let read_back = read_lock_file(&path).unwrap();

    let meta = read_back.skills.get("owner/monorepo/skill-a").unwrap();
    assert_eq!(meta.subdirectory, Some("skill-a".to_string()));
}

#[test]
fn test_write_overwrites_existing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(".skills.lock");

    let mut lf1 = LockFile::new();
    lf1.skills.insert(
        "owner/repo".to_string(),
        SkillMeta::new("owner/repo", "sha1"),
    );
    write_lock_file_atomic(&path, &lf1).unwrap();

    let mut lf2 = LockFile::new();
    lf2.skills.insert(
        "owner/repo".to_string(),
        SkillMeta::new("owner/repo", "sha2"),
    );
    write_lock_file_atomic(&path, &lf2).unwrap();

    let read_back = read_lock_file(&path).unwrap();
    assert_eq!(read_back.skills.get("owner/repo").unwrap().sha, "sha2");
}
