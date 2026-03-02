---
phase: 03-sync-engine
plan: 02
subsystem: sync
tags: [rust, dialoguer, orphan-detection, sync-pipeline, cli]

# Dependency graph
requires:
  - phase: 03-sync-engine-01
    provides: execute_install, execute_update, execute_remove, skill_dir_name git subprocess operations
  - phase: 02-github-api
    provides: GitHubClient, resolver SyncAction types, discover_token
provides:
  - cleanup_stale_temp_dirs: removes leftover .skills-sync-tmp-* dirs before each sync
  - scan_orphans: detects installed skills not in current manifest
  - execute_sync_plan: full orchestration (cleanup -> install/update -> orphan scan -> batch prompt -> removal)
  - End-to-end pipeline in main.rs: manifest -> API -> resolve -> sync -> orphan cleanup
affects: [04-global-install, future-phases]

# Tech tracking
tech-stack:
  added: [dialoguer 0.12 (interactive confirmation prompts)]
  patterns:
    - Sequential skill sync with per-skill warn-and-continue error handling
    - Batch orphan removal with single Y/n prompt (not per-skill prompts)
    - Non-TTY safe: interact_opt() defaults to false on non-interactive terminals
    - Token cloned before GitHubClient construction to avoid move

key-files:
  created: []
  modified:
    - Cargo.toml (added dialoguer 0.12)
    - src/sync.rs (cleanup_stale_temp_dirs, OrphanSkill, scan_orphans, prompt_orphan_removal, execute_sync_plan)
    - src/main.rs (branch_map, manifest_repos, sync_token, execute_sync_plan call)

key-decisions:
  - "Always call execute_sync_plan regardless of action count — function handles no-op case with cleanup and orphan scan"
  - "Resolver Remove actions and global orphan scan results combined into single batch prompt"
  - "Token cloned before GitHubClient::new() since client takes ownership; sync engine reuses cloned copy"
  - "manifest_repos.insert() placed before early continues so repos are tracked even on API errors"

patterns-established:
  - "Warn-and-skip pattern: per-skill failures emit warning, increment failed count, continue remaining skills"
  - "Batch confirmation: collect all removals first, single Y/n prompt, then remove all confirmed"
  - "Non-TTY safety: interact_opt() returns Err on non-interactive, mapped to false (no action)"

requirements-completed: [SYNC-04, SYNC-06, ERR-03]

# Metrics
duration: 12min
completed: 2026-03-02
---

# Phase 3 Plan 02: Sync Engine Orchestration Summary

**Full end-to-end sync pipeline: manifest -> API -> resolve -> execute_sync_plan with orphan detection, batch removal prompt, and dialoguer-based confirmation**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-02T10:03:58Z
- **Completed:** 2026-03-02T10:16:00Z
- **Tasks:** 2
- **Files modified:** 3 (Cargo.toml, src/sync.rs, src/main.rs)

## Accomplishments

- Extended sync.rs with orphan detection, stale temp cleanup, and full sync orchestration
- Wired execute_sync_plan into main.rs completing the end-to-end pipeline
- Batch orphan removal with single Y/n prompt via dialoguer (non-TTY safe)
- Per-skill failures warn and continue; non-zero exit code if any skill fails (ERR-03)
- Target directory defaults to .claude/skills (SYNC-06)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add orphan scan, temp cleanup, and sync orchestration to sync.rs** - `0e1fe13` (feat)
2. **Task 2: Wire sync engine into main.rs for end-to-end pipeline** - `c8cbd18` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `/Users/benvds/code/skills-sync/Cargo.toml` - Added dialoguer = "0.12" dependency
- `/Users/benvds/code/skills-sync/src/sync.rs` - Added cleanup_stale_temp_dirs, OrphanSkill struct, scan_orphans, prompt_orphan_removal, execute_sync_plan
- `/Users/benvds/code/skills-sync/src/main.rs` - Added branch_map/manifest_repos tracking, sync_token clone, execute_sync_plan call

## Decisions Made

- Always call `execute_sync_plan` unconditionally — the function already handles empty/skip-only action lists gracefully (just does cleanup and orphan scan). Simpler than conditional logic.
- Resolver `Remove` actions and global orphan scan merged into one list for a single batch prompt — avoids two separate confirmation dialogs.
- Token cloned before `GitHubClient::new()` because the client takes ownership; the clone is passed to the sync engine.
- `manifest_repos.insert()` placed at the very top of the per-repo loop, before any `continue` statements, so repos are tracked in the set even when API calls fail.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Full sync pipeline complete: install, update, remove, and orphan detection all wired end-to-end
- Phase 4 (Global Install) can add `--global` flag resolution before constructing `target_dir`; the sync engine itself needs no changes
- All 76 tests passing across all test suites

## Self-Check: PASSED

- FOUND: .planning/phases/03-sync-engine/03-02-SUMMARY.md
- FOUND: src/sync.rs (cleanup_stale_temp_dirs, scan_orphans, execute_sync_plan)
- FOUND: src/main.rs (execute_sync_plan call, branch_map, manifest_repos)
- FOUND: Cargo.toml (dialoguer = "0.12")
- FOUND: commit 0e1fe13 (Task 1)
- FOUND: commit c8cbd18 (Task 2)

---
*Phase: 03-sync-engine*
*Completed: 2026-03-02*
