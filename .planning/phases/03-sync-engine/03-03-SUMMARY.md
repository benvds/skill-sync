---
phase: 03-sync-engine
plan: 03
subsystem: cli
tags: [clap, dirs, cli-flags, path-resolution]

# Dependency graph
requires:
  - phase: 03-sync-engine/03-02
    provides: "sync pipeline with execute_sync_plan and load_local_metadata wired in main.rs"
provides:
  - "--global CLI flag on Cli struct routing sync to ~/.claude/skills/"
  - "Conditional target_dir resolution using dirs::home_dir() in main.rs"
  - "load_local_metadata accepts target_dir parameter instead of hardcoded path"
affects: [phase-04, integration-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Resolve path early before loops so all consumers share one resolved value"
    - "Use ok_or_else with SyncFailed variant for Option -> Result conversion on critical env values"

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/main.rs

key-decisions:
  - "Use existing SyncFailed error variant (not a new variant) for home_dir() None case to avoid conflicting From impls"
  - "Resolve target_dir_buf before the per-repo loop so load_local_metadata and execute_sync_plan share same resolved path"
  - "load_local_metadata refactored to accept target_dir as parameter rather than closing over hardcoded path"

patterns-established:
  - "CLI flag gates path resolution: cli.global conditional switches between local and global targets"
  - "PathBuf::from for local path, dirs::home_dir().join() for global path — both coerced to &Path via as_path()"

requirements-completed: [SYNC-01, SYNC-02, SYNC-04, SYNC-06, ERR-03, ERR-04]

# Metrics
duration: 2min
completed: 2026-03-02
---

# Phase 3 Plan 3: --global Flag and Conditional target_dir Resolution Summary

**--global CLI flag added to Cli struct routing sync to ~/.claude/skills/ via dirs::home_dir(), completing SYNC-06**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-02T10:21:06Z
- **Completed:** 2026-03-02T10:23:17Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Added `pub global: bool` field to Cli struct with `#[arg(long)]` and descriptive doc comment
- Resolved target_dir conditionally in main.rs before per-repo loop using dirs::home_dir() when --global is set
- Updated load_local_metadata to accept target_dir as parameter, eliminating hardcoded ".claude/skills" path
- Returns clear SyncFailed error if home_dir() cannot be resolved when --global is set
- All 76 existing tests pass with no regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Add --global flag to Cli struct in src/cli.rs** - `77a94fb` (feat)
2. **Task 2: Resolve target_dir conditionally in src/main.rs** - `4cc0fa1` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/cli.rs` - Added `pub global: bool` field with `#[arg(long)]` and doc comment
- `src/main.rs` - Resolved target_dir_buf before per-repo loop, updated load_local_metadata signature and call site

## Decisions Made
- Used existing `SkillsSyncError::SyncFailed` variant (not a new one) for the home_dir() None case — avoids conflicting `From<io::Error>` impls that would arise from a new error variant
- target_dir_buf resolved before the per-repo loop so both load_local_metadata (inside loop) and execute_sync_plan (after loop) share the same resolved path

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- SYNC-06 fully satisfied: local default (.claude/skills/) works without --global; global target (~/.claude/skills/) works with --global
- No regressions in existing 76 tests
- Phase 3 sync engine complete and ready for Phase 4

---
*Phase: 03-sync-engine*
*Completed: 2026-03-02*
