---
phase: quick-1
plan: 01
subsystem: metadata
tags: [rust, toml, lock-file, metadata, sync-engine]

# Dependency graph
requires:
  - phase: 03-sync-engine
    provides: "per-skill sibling .lock files, sync pipeline, execute_install/update/remove"
provides:
  - "Consolidated .skills.lock file next to manifest containing all installed skill metadata"
  - "LockFile struct with HashMap<String, SkillMeta> keyed by skill_key"
  - "read_lock_file / write_lock_file_atomic functions for consolidated lock file"
  - "lock_path_for_manifest helper to derive lock path from manifest path"
  - "Updated execute_install/update/remove accepting lock_path and &mut LockFile"
  - "Updated scan_orphans reading from LockFile instead of scanning *.lock files"
affects: [sync-engine, metadata, future-phases]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Consolidated lock file pattern: single .skills.lock next to manifest, not N per-skill files"
    - "LockFile as shared mutable state passed through sync pipeline"
    - "read_lock_file returns Ok(empty) on missing file (first-run safe)"

key-files:
  created: []
  modified:
    - src/metadata.rs
    - src/sync.rs
    - src/main.rs
    - tests/metadata_tests.rs
    - tests/sync_tests.rs

key-decisions:
  - "Lock file path derived from manifest path via with_extension('lock') — .skills.toml -> .skills.lock"
  - "schema_version moved from SkillMeta to LockFile only — per-entry schema version removed"
  - "LockFile passed as &mut through entire sync pipeline for crash-safe per-operation writes"
  - "read_lock_file returns Ok(LockFile::new()) on NotFound — no special first-run handling needed"
  - "scan_orphans reads from LockFile parameter, no longer scans filesystem for *.lock files"
  - "execute_remove signature changed: accepts lock_path + &mut LockFile + skill_key_str"

patterns-established:
  - "Single source of truth: all skill metadata in one .skills.lock file"
  - "Atomic writes: write_lock_file_atomic uses temp-file + rename pattern"
  - "Pipeline state: LockFile read once at execute_sync_plan start, mutated per operation"

requirements-completed: [LOCK-CONSOLIDATE]

# Metrics
duration: 5min
completed: 2026-03-02
---

# Quick Task 1: Consolidate Lock Files into Single .skills.lock Summary

**Single .skills.lock file next to manifest replacing N per-skill sibling *.lock files, with LockFile struct (HashMap of all skill metadata) threaded through the entire sync pipeline**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-02T14:50:21Z
- **Completed:** 2026-03-02T14:55:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Replaced N per-skill sibling `.lock` files with a single `.skills.lock` consolidated lock file next to the manifest
- Added `LockFile` struct with `schema_version` and `skills: HashMap<String, SkillMeta>` keyed by skill_key
- Updated entire sync pipeline: execute_install/update/remove now accept and mutate a shared LockFile
- Updated orphan detection to read from LockFile instead of scanning filesystem for `*.lock` files
- Added `read_lock_file` (returns empty LockFile on missing file, safe for first run) and `write_lock_file_atomic`
- Removed `schema_version` from `SkillMeta` (moved to `LockFile`), removed `meta_path_for_skill`, `META_EXTENSION`
- All 95 tests pass, clean build, no new clippy warnings

## Task Commits

1. **Task 1: LockFile struct, consolidated read/write, sync.rs + main.rs update** - `114f8de` (feat)
2. **Task 2: Clean up and verify** - no additional changes needed; code was already clean

## Files Created/Modified

- `src/metadata.rs` - Added LockFile struct, read_lock_file, write_lock_file_atomic, lock_path_for_manifest; removed META_EXTENSION, meta_path_for_skill, schema_version from SkillMeta
- `src/sync.rs` - Updated execute_install/update/remove/scan_orphans/execute_sync_plan to use consolidated LockFile
- `src/main.rs` - Derives lock_path from manifest_path, reads LockFile once, passes through pipeline; updated load_local_metadata to read from LockFile
- `tests/metadata_tests.rs` - New LockFile tests (new, roundtrip, nonexistent, serialization, insert/remove); removed schema_version refs from SkillMeta tests
- `tests/sync_tests.rs` - Updated execute_remove tests to use new signature (lock_path + LockFile + skill_key)

## Decisions Made

- Lock file path derived from manifest path using `with_extension("lock")` — intuitive, keeps lock file next to manifest
- `schema_version` moved from `SkillMeta` to `LockFile` only — version lives at the file level, not per-entry
- `LockFile` passed as `&mut` through entire sync pipeline for crash-safe writes after each operation
- `read_lock_file` returns `Ok(LockFile::new())` on `NotFound` — first run works without special-casing
- `scan_orphans` now accepts `&LockFile` parameter instead of scanning filesystem — faster and simpler
- Added `#[allow(clippy::too_many_arguments)]` to `execute_install` and `execute_update` (8 args each, necessary for lockfile threading)
- Fixed `matches!` macro improvement in `prompt_orphan_removal` (pre-existing issue, fixed since we rewrote the file)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Correctness] Fixed `io_other_error` clippy lint in new write_lock_file_atomic**
- **Found during:** Task 2 (clean-up verification)
- **Issue:** Used `std::io::Error::new(ErrorKind::Other, e)` where `std::io::Error::other(e)` should be used
- **Fix:** Changed to `std::io::Error::other(e)` idiomatic form
- **Files modified:** src/metadata.rs
- **Verification:** clippy passes without this warning
- **Committed in:** 114f8de (part of task 1 commit)

**2. [Rule 3 - Blocking] Added `#[allow(clippy::too_many_arguments)]` to execute_install/execute_update**
- **Found during:** Task 2 (clean-up verification)
- **Issue:** Both functions have 8 arguments (1 over clippy's default limit of 7) due to adding lock_path + lock_file params
- **Fix:** Added `#[allow(clippy::too_many_arguments)]` to both functions — structural change required by plan, not reducible
- **Files modified:** src/sync.rs
- **Verification:** clippy no longer reports these as errors
- **Committed in:** 114f8de (part of task 1 commit)

**3. [Rule 1 - Bug] Fixed matches! macro pattern in prompt_orphan_removal**
- **Found during:** Task 2 (clean-up verification)
- **Issue:** match expression could be simplified to `matches!` macro (pre-existing in original sync.rs, but in file we rewrote)
- **Fix:** Converted match to `matches!(..., Ok(Some(true)))`
- **Files modified:** src/sync.rs
- **Verification:** clippy no longer reports this warning
- **Committed in:** 114f8de (part of task 1 commit)

---

**Total deviations:** 3 auto-fixed (1 clippy style fix, 1 allow annotation, 1 matches! macro fix)
**Impact on plan:** All fixes necessary for correctness and clean compilation. No scope creep.

## Issues Encountered

None - implementation followed the plan exactly. The pre-existing `manifest.rs` clippy errors (`useless_conversion` on `usize`) were confirmed pre-existing and are out of scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Consolidated lock file infrastructure complete
- Target directory is now free of metadata files (only skill directories)
- Single `.skills.lock` alongside `.skills.toml` is easy to find and version-control
- Future phases can read/write skill metadata through the clean LockFile API

---
*Quick Task: 1*
*Completed: 2026-03-02*
