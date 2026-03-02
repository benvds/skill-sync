---
phase: quick-1
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/metadata.rs
  - src/sync.rs
  - src/main.rs
  - tests/metadata_tests.rs
  - tests/sync_tests.rs
autonomous: true
requirements: [LOCK-CONSOLIDATE]

must_haves:
  truths:
    - "A single .skills.lock file is created next to the manifest file"
    - "The lock file contains entries for all installed skills keyed by skill key"
    - "Per-skill sibling .lock files are no longer created"
    - "Reading local metadata loads from the single lock file instead of scanning *.lock files"
    - "Orphan detection reads from the single lock file"
    - "Install, update, and remove operations update the single lock file atomically"
  artifacts:
    - path: "src/metadata.rs"
      provides: "LockFile struct with HashMap<String, SkillMeta>, read/write functions for consolidated lock file"
    - path: "src/sync.rs"
      provides: "Updated sync operations that accept lock_path and mutate a shared LockFile"
    - path: "src/main.rs"
      provides: "Lock file path derived from manifest path, passed through sync pipeline"
  key_links:
    - from: "src/main.rs"
      to: "src/metadata.rs"
      via: "read_lock_file / write_lock_file_atomic"
      pattern: "read_lock_file|write_lock_file_atomic"
    - from: "src/sync.rs"
      to: "src/metadata.rs"
      via: "LockFile struct mutation"
      pattern: "LockFile"
    - from: "src/main.rs"
      to: "src/sync.rs"
      via: "execute_sync_plan receives lock_path"
      pattern: "execute_sync_plan.*lock_path"
---

<objective>
Consolidate per-skill sibling lock files into a single `.skills.lock` file next to the manifest.

Purpose: Instead of N separate `.lock` files in the skills target directory (one per installed skill), produce a single lock file alongside the manifest file (e.g., `.skills.lock` next to `.skills.toml`). This is cleaner, keeps the target directory free of metadata files, and makes the lock file easy to find and version-control.

Output: Modified metadata.rs, sync.rs, main.rs, and updated tests.
</objective>

<execution_context>
@/Users/benvds/.claude/get-shit-done/workflows/execute-plan.md
@/Users/benvds/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/metadata.rs
@src/sync.rs
@src/main.rs
@src/error.rs
@tests/metadata_tests.rs
@tests/sync_tests.rs

<interfaces>
<!-- Current metadata.rs exports that will change -->
From src/metadata.rs:
```rust
pub const META_FILENAME: &str = ".skills.lock";
pub const META_EXTENSION: &str = "lock";
pub fn meta_path_for_skill(target_dir: &Path, dir_name: &str) -> PathBuf;
pub struct SkillMeta { schema_version, source, subdirectory, sha, installed_at, updated_at }
pub fn write_metadata_atomic(path: &Path, meta: &SkillMeta) -> std::io::Result<()>;
pub fn read_metadata(path: &Path) -> std::io::Result<SkillMeta>;
```

From src/sync.rs:
```rust
pub fn skill_dir_name(repo: &str, subdirectory: &Option<String>) -> String;
pub fn execute_install(repo, subdirectory, remote_sha, branch, target_dir, token) -> Result<()>;
pub fn execute_update(repo, subdirectory, new_sha, branch, target_dir, token) -> Result<()>;
pub fn execute_remove(skill_path: &Path, lock_path: Option<&Path>) -> Result<()>;
pub fn scan_orphans(target_dir: &Path, manifest_repos: &HashSet<String>) -> Vec<OrphanSkill>;
pub fn execute_sync_plan(actions, target_dir, token, branch_map, manifest_repos) -> Result<()>;
```

From src/main.rs:
```rust
fn load_local_metadata(repo: &str, target_dir: &Path) -> HashMap<String, SkillMeta>;
// Scans target_dir for *.lock files matching repo
```

From src/resolver.rs:
```rust
pub fn skill_key(repo: &str, subdirectory: &Option<String>) -> String;
// "owner/repo" or "owner/repo/subdir"
```
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Add LockFile struct and consolidated read/write to metadata.rs, update sync.rs and main.rs</name>
  <files>src/metadata.rs, src/sync.rs, src/main.rs, tests/metadata_tests.rs, tests/sync_tests.rs</files>
  <behavior>
    - Test: LockFile::new() creates empty skills HashMap
    - Test: LockFile with multiple entries serializes to TOML with `[skills."owner/repo"]` sections
    - Test: LockFile roundtrip (write then read) preserves all SkillMeta fields for multiple skills
    - Test: write_lock_file_atomic creates file atomically (temp + rename)
    - Test: read_lock_file on nonexistent path returns empty LockFile (not error)
    - Test: Inserting and removing entries from LockFile works correctly
    - Test: LockFile serialization omits subdirectory when None (skip_serializing_if)
    - Test: execute_sync_plan with consolidated lock file creates single .skills.lock next to manifest
  </behavior>
  <action>
**In src/metadata.rs:**

1. Add a new `LockFile` struct:
```rust
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct LockFile {
    pub schema_version: u32,
    pub skills: HashMap<String, SkillMeta>,
}
```
Where keys are the skill_key format ("owner/repo" or "owner/repo/subdir"). schema_version = 1.

2. Add `LockFile::new()` that creates an empty LockFile with schema_version=1.

3. Add `read_lock_file(path: &Path) -> std::io::Result<LockFile>` — reads and deserializes the consolidated lock file. If the file does not exist, returns `Ok(LockFile::new())` (empty, not an error). This is important because on first run there is no lock file yet.

4. Add `write_lock_file_atomic(path: &Path, lock_file: &LockFile) -> std::io::Result<()>` — same atomic temp+rename pattern as existing `write_metadata_atomic`.

5. Remove `SkillMeta.schema_version` field — the schema version now lives on `LockFile` only. Update `SkillMeta::new()` accordingly.

6. Remove `META_EXTENSION` const and `meta_path_for_skill` function (no longer needed — there are no per-skill lock files). Keep `META_FILENAME` as `".skills.lock"` for the consolidated lock file name.

7. Keep `read_metadata` and `write_metadata_atomic` for now but mark them with `#[allow(dead_code)]` or remove if no longer used after other changes.

**In src/sync.rs:**

1. Change `execute_install` signature: add `lock_path: &Path` and `lock_file: &mut LockFile` parameters. After successful git clone + rename, instead of writing a per-skill lock file, insert the SkillMeta into `lock_file.skills` using `skill_key(repo, subdirectory)` as key, then call `write_lock_file_atomic(lock_path, lock_file)`. Remove the per-skill `write_metadata_atomic` call.

2. Change `execute_update` similarly: add `lock_path: &Path` and `lock_file: &mut LockFile`. Read `installed_at` from `lock_file.skills.get(&key)` instead of reading a per-skill lock file. After re-install, patch `installed_at` back in the lock_file entry.

3. Change `execute_remove` signature: replace `lock_path: Option<&Path>` with `lock_path: &Path`, `lock_file: &mut LockFile`, and `skill_key: &str`. After removing the skill directory, remove the entry from `lock_file.skills` and write the updated lock file. No more per-skill lock file deletion.

4. Update `scan_orphans`: instead of reading per-skill lock files from target_dir, accept `lock_file: &LockFile` parameter. Iterate `lock_file.skills` entries where `meta.source` is not in `manifest_repos` and the skill directory exists on disk.

5. Update `execute_sync_plan` signature: add `lock_path: &Path`. At the start, read the lock file with `read_lock_file(lock_path)`. Pass the mutable `lock_file` to install/update/remove calls. The lock file gets written after each individual operation for crash safety.

6. Update internal `prompt_orphan_removal` and orphan batch removal to use the new `execute_remove` signature.

**In src/main.rs:**

1. Derive `lock_path` from `manifest_path`: replace the extension with `.lock` (e.g., `.skills.toml` -> `.skills.lock`). Use `manifest_path.with_extension("lock")`.

2. Replace `load_local_metadata` function: instead of scanning `*.lock` files in target_dir, read the consolidated lock file once with `read_lock_file(&lock_path)` and filter entries by repo source. The function should accept `&LockFile` instead of reading from disk.

3. Pass `lock_path` to `execute_sync_plan`.

4. Remove the per-skill lock file scanning logic entirely.

**In tests/metadata_tests.rs:**

1. Add tests for `LockFile::new()`, serialization roundtrip, read from nonexistent file returning empty.
2. Update existing tests that reference `schema_version` on `SkillMeta` — it no longer has that field.
3. Remove tests for `meta_path_for_skill` if it's removed.

**In tests/sync_tests.rs:**

1. Update `execute_remove_with_lock_file` test to use the new signature (pass LockFile and skill_key instead of lock file path).
2. Update `meta_path_for_skill` tests or remove them.
3. Keep `skill_dir_name` and `build_clone_url` tests unchanged.

**Lock file format example (TOML):**
```toml
schema_version = 1

[skills."anthropics/skills/skills/skill-creator"]
source = "anthropics/skills"
subdirectory = "skills/skill-creator"
sha = "3d59511518591fa82e6cfcf0438d68dd5dad3e76"
installed_at = "2026-03-02T14:43:29.907609Z"
updated_at = "2026-03-02T14:43:29.907609Z"

[skills."github/awesome-copilot/skills/gh-cli"]
source = "github/awesome-copilot"
subdirectory = "skills/gh-cli"
sha = "abc123def456"
installed_at = "2026-03-02T14:44:00.000000Z"
updated_at = "2026-03-02T14:44:00.000000Z"
```
  </action>
  <verify>
    <automated>cd /Users/benvds/code/skills-sync && cargo test 2>&1</automated>
  </verify>
  <done>
    - LockFile struct exists with schema_version and skills HashMap
    - read_lock_file returns empty LockFile for missing file
    - write_lock_file_atomic writes atomically
    - execute_install/update/remove operate on shared LockFile
    - execute_sync_plan reads/writes single lock file
    - main.rs derives lock_path from manifest_path
    - No per-skill .lock files are created
    - All tests pass (cargo test)
  </done>
</task>

<task type="auto">
  <name>Task 2: Clean up dead code and verify end-to-end</name>
  <files>src/metadata.rs, src/sync.rs, tests/metadata_tests.rs, tests/sync_tests.rs</files>
  <action>
1. Remove any dead code left from Task 1: `meta_path_for_skill`, `META_EXTENSION`, `read_metadata`, `write_metadata_atomic` if they are no longer called anywhere. Run `cargo build 2>&1 | grep "unused"` to find remaining dead code warnings.

2. Remove `SkillMeta.schema_version` field completely if not already done — schema_version lives on LockFile only. Ensure `Serialize`/`Deserialize` still work for SkillMeta without it.

3. Run `cargo clippy -- -D warnings` to ensure no warnings.

4. Run `cargo test` to confirm all tests still pass after cleanup.

5. Verify the compiled binary works: `cargo build` succeeds with no warnings.
  </action>
  <verify>
    <automated>cd /Users/benvds/code/skills-sync && cargo clippy -- -D warnings 2>&1 && cargo test 2>&1</automated>
  </verify>
  <done>
    - No dead code warnings from cargo build or clippy
    - No clippy warnings
    - All tests pass
    - Binary compiles cleanly
  </done>
</task>

</tasks>

<verification>
1. `cargo test` — all existing and new tests pass
2. `cargo clippy -- -D warnings` — no warnings
3. `cargo build` — compiles cleanly
4. No `*.lock` files in target_dir after a sync (only directories)
5. Single `.skills.lock` file created next to manifest
</verification>

<success_criteria>
- Single `.skills.lock` file is created next to the manifest file containing all skill metadata
- Per-skill sibling `.lock` files are no longer created in the target directory
- Lock file uses TOML with `[skills."key"]` sections for each installed skill
- All existing tests updated and passing, new tests for LockFile added
- No clippy warnings, clean compilation
</success_criteria>

<output>
After completion, create `.planning/quick/1-consolidate-lock-files-into-single-lock-/1-SUMMARY.md`
</output>
