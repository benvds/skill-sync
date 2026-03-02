---
phase: 03-sync-engine
plan: 05
subsystem: sync
tags: [rust, metadata, lock-files, sibling-files, atomic-writes]

# Dependency graph
requires:
  - phase: 03-sync-engine
    provides: execute_install, execute_update, execute_remove, scan_orphans, load_local_metadata functions
provides:
  - meta_path_for_skill() helper computing sibling lock file paths
  - META_EXTENSION constant for *.lock file extension
  - execute_remove accepts optional lock_path for sibling lock cleanup
  - execute_install writes lock files at target_dir/dir_name.lock (sibling to skill dir)
  - execute_update reads/writes sibling lock files for installed_at preservation
  - scan_orphans reads sibling lock files instead of files inside skill directories
  - load_local_metadata scans *.lock files at target_dir level (not inside subdirs)
affects: [04-cli, future-phases]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Sibling lock file pattern: metadata stored at target_dir/owner--repo.lock alongside target_dir/owner--repo/
    - Best-effort cleanup: lock file removal on execute_remove is best-effort (failure does not block skill removal)
    - META_EXTENSION constant drives extension-based scanning in load_local_metadata

key-files:
  created: []
  modified:
    - src/metadata.rs
    - src/sync.rs
    - src/main.rs
    - tests/sync_tests.rs

key-decisions:
  - "Lock files moved from inside skill directories (.claude/skills/owner--repo/.skills.lock) to sibling files at target_dir level (.claude/skills/owner--repo.lock) based on UAT user feedback"
  - "execute_remove signature updated to accept Option<&Path> lock_path - None for backward compat, Some for sibling cleanup"
  - "Lock file removal in execute_remove is best-effort - skill dir removal succeeds even if lock file deletion fails"

patterns-established:
  - "Sibling lock files: metadata stored as dir_name.lock alongside skill directory, not inside it"
  - "meta_path_for_skill(target_dir, dir_name) is the canonical way to compute lock file paths"

requirements-completed: [SYNC-01, SYNC-02, SYNC-04, ERR-03, ERR-04]

# Metrics
duration: 2min
completed: 2026-03-02
---

# Phase 03 Plan 05: Sibling Lock File Placement Summary

**Moved per-skill metadata from inside skill directories to sibling *.lock files at the target directory level, with meta_path_for_skill() helper used across install, update, remove, orphan scan, and metadata loading.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-02T14:27:50Z
- **Completed:** 2026-03-02T14:29:50Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Added `meta_path_for_skill()` helper and `META_EXTENSION` constant in `src/metadata.rs`
- Updated `execute_install`, `execute_update`, `scan_orphans` in `src/sync.rs` to use sibling lock files
- Updated `execute_remove` to accept optional lock path and delete sibling lock file on removal
- Updated `load_local_metadata` in `src/main.rs` to scan `*.lock` files at the target_dir level
- Added 3 new tests: `meta_path_single`, `meta_path_monorepo`, `execute_remove_with_lock_file`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add meta_path_for_skill helper and update sync operations** - `bc47178` (feat)
2. **Task 2: Update load_local_metadata to read sibling lock files** - `58de088` (feat)

## Files Created/Modified
- `src/metadata.rs` - Added `META_EXTENSION` constant and `meta_path_for_skill()` helper function
- `src/sync.rs` - Updated execute_install, execute_update, execute_remove, scan_orphans, execute_sync_plan
- `src/main.rs` - Updated load_local_metadata to scan *.lock files at target_dir level
- `tests/sync_tests.rs` - Updated execute_remove calls to pass lock_path; added meta_path and lock file tests

## Decisions Made
- Lock files moved to sibling position at target_dir level based on UAT feedback where user expected metadata alongside skill directories, not buried inside cloned content
- `execute_remove` uses `Option<&Path>` for lock_path to keep the signature backward compatible; pass `None` when no lock file cleanup is needed
- Lock file removal is best-effort: `let _ = std::fs::remove_file(lp)` — skill directory removal is the primary success condition

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed type inference ambiguity in execute_remove with ?**
- **Found during:** Task 1 (execute_remove signature update)
- **Issue:** Adding `?` after `.map_err(|e| { ... .into() })` caused `E0283` type inference error because the compiler couldn't determine which `From` impl to use with `ErrReport`
- **Fix:** Removed the intermediate `.into()` by inlining the error construction: `.map_err(|e| SkillsSyncError::RemoveFailed { ... })?` — the `?` operator handles the conversion via the `From` impl for `SkillsSyncError`
- **Files modified:** `src/sync.rs`
- **Verification:** `cargo test --test sync_tests` passes 10 tests
- **Committed in:** bc47178 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug)
**Impact on plan:** Required fix for compilation; no scope creep.

## Issues Encountered
None beyond the auto-fixed type inference issue above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Sibling lock file pattern fully implemented and tested
- Phase 4 (CLI/distribution) can build on the updated sync engine
- All 79 tests pass; no regressions introduced

---
*Phase: 03-sync-engine*
*Completed: 2026-03-02*
