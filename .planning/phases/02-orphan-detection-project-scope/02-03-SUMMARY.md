---
phase: 02-orphan-detection-project-scope
plan: 03
subsystem: executor, reporter
tags: typescript, subprocess, cli-flags, orphan-removal

requires:
  - phase: 02-orphan-detection-project-scope
    provides: Extended SyncOptions (yes, verbose, agent), SyncResult (removed) from types.ts
provides:
  - removeSkills function for batch npx skills remove calls
  - Verbose-gated printCommand in executeSync
  - --agent passthrough to npx skills add and remove
  - Reporter methods for orphan/removal output
affects: [02-04]

tech-stack:
  added: []
  patterns: [verbose gating pattern, batch subprocess removal]

key-files:
  created: []
  modified: [src/executor.ts, src/reporter.ts, test/executor.test.ts, test/reporter.test.ts, bin/skills-sync.ts]

key-decisions:
  - "removeSkills batches all skill names in single npx skills remove call — one subprocess for N orphans"
  - "printCommand gated by if (options.verbose) — changes Phase 1 always-on to Phase 2 opt-in"
  - "printSummary signature changed to (installed, removed, unchanged) — all call sites updated"
  - "printDryRunSummary signature changed to (toInstall, toRemove, unchanged)"

patterns-established:
  - "Verbose gating: reporter.printCommand only called when options.verbose is true"
  - "Batch removal: single npx subprocess for all orphan skill names"

requirements-completed: [SYNC-04, FLAG-04, FLAG-05]

duration: 3min
completed: 2026-03-01
---

# Plan 02-03: Executor Removal + Reporter Orphan Methods Summary

**removeSkills function with batch npx skills remove, verbose-gated command printing, --agent passthrough, and reporter orphan/removal methods**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-01
- **Completed:** 2026-03-01
- **Tasks:** 1
- **Files modified:** 5

## Accomplishments
- removeSkills batches all skill names in single npx skills remove call with --yes/--global/--agent
- printCommand gated behind options.verbose (Phase 1 always-on changed to opt-in)
- --agent threaded to npx skills add args
- Reporter: printOrphans, printRemoving, printRemoved, warnRemoveFailed, warnSkippingOrphans, printPreSyncPreview
- printSummary and printDryRunSummary include removed/toRemove count

## Task Commits

Each task was committed atomically:

1. **Task 1: Reporter + executor extensions** - `b4bddd9` (feat)

## Files Created/Modified
- `src/executor.ts` - removeSkills function, verbose gate, agent passthrough
- `src/reporter.ts` - 6 new methods, updated printSummary/printDryRunSummary signatures
- `test/executor.test.ts` - 13 new tests for removeSkills, verbose, agent
- `test/reporter.test.ts` - 9 new tests for orphan/removal methods and updated signatures
- `bin/skills-sync.ts` - Updated printSummary/printDryRunSummary call sites

## Decisions Made
- Batch removal in single subprocess (npx skills remove skill-a skill-b --yes) for efficiency
- printCommand verbose gate applied consistently in both executeSync and removeSkills

## Deviations from Plan
None - plan executed exactly as written

## Issues Encountered
- TypeScript strict mode required explicit type annotation for mock.calls.map callback parameter

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- removeSkills ready for CLI integration (02-04)
- All reporter methods ready for CLI wiring

---
*Phase: 02-orphan-detection-project-scope*
*Completed: 2026-03-01*
