---
phase: 04-cli-ux
plan: 01
subsystem: cli
tags: [clap, dry-run, force, cli-flags, orphan-detection]

# Dependency graph
requires:
  - phase: 03-sync-engine
    provides: execute_sync_plan, scan_orphans, prompt_orphan_removal
  - phase: quick-1
    provides: lock_path_for_manifest, consolidated lock file pattern
provides:
  - All CLI flags (--dry-run, --force, --local, --version, --help) parsing correctly
  - --dry-run short-circuit with +/~/- prefixes and exit code 2 for pending changes
  - --force auto-confirm for CI usage bypassing orphan prompt
  - --global/--local mutual exclusivity enforced by clap before any network calls
  - Global orphan detection included in dry-run output
affects: [04-cli-ux plan 02, distribution, CI scripts]

# Tech tracking
tech-stack:
  added: []
  patterns: [clap conflicts_with for mutual exclusivity, std::process::exit(2) for dry-run CI contract]

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/main.rs
    - src/sync.rs

key-decisions:
  - "Dry-run scans global orphans before short-circuit so preview output is complete (includes orphans)"
  - "force param added to prompt_orphan_removal (private fn) and execute_sync_plan (public fn) for CI usage"
  - "conflicts_with on local field (not flag name) per clap derive convention for mutual exclusivity"
  - "std::process::exit(2) used in dry-run (safe — no resources to clean up at that point)"

patterns-established:
  - "Orphan scan before dry-run check ensures complete preview output"
  - "clap conflicts_with attribute handles mutual exclusivity without runtime checks"

requirements-completed: [CLI-01, CLI-02, CLI-04, CLI-05, CLI-06]

# Metrics
duration: 5min
completed: 2026-03-02
---

# Phase 4 Plan 01: CLI Flags and Dry-run Summary

**Full CLI flag surface with --dry-run (exit 2 on pending changes), --force (CI auto-confirm), --local/--global mutual exclusivity, and --version from Cargo.toml**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-02T16:54:52Z
- **Completed:** 2026-03-02T16:56:39Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added dry_run, force, local fields to Cli struct with --version auto-reading from Cargo.toml
- --global --local mutual exclusivity enforced by clap before any network calls
- --dry-run short-circuit with +/~/- action prefixes, exit 2 when changes pending, exit 0 when up-to-date
- Global orphans scanned before dry-run check so preview includes removed-repo orphans
- --force threads through to prompt_orphan_removal, bypassing dialoguer prompt for CI usage
- All 95 existing tests continue to pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Add CLI flags and mutual exclusivity to cli.rs** - `046d013` (feat)
2. **Task 2: Wire dry-run short-circuit and force passthrough** - `7e151a1` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/cli.rs` - Added dry_run, force, local fields; version attribute on #[command]; improved verbose doc comment
- `src/main.rs` - Added global orphan scan before dry-run check; dry-run short-circuit with +/~/- output and exit 2; pass cli.force to execute_sync_plan
- `src/sync.rs` - Added force param to prompt_orphan_removal (auto-returns true when force); added force param to execute_sync_plan signature; updated internal call

## Decisions Made
- Dry-run reads lock file a second time for orphan scan (separate from the one used for resolver) — clean separation, pre-sync state is correct for preview
- execute_sync_plan keeps its own internal orphan scan intact after installs/updates complete (catches slightly different state than pre-sync scan)
- Global orphan Remove actions added to all_actions list to unify dry-run output, with dedup check against resolver Remove actions

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All six CLI flags (--dry-run, --force, --verbose, --local, --version, --help) fully operational
- Ready for Plan 02: color-coded output and progress feedback
- --force enables CI drift detection: `skills-sync --dry-run || echo "skills out of sync"`

---
*Phase: 04-cli-ux*
*Completed: 2026-03-02*
