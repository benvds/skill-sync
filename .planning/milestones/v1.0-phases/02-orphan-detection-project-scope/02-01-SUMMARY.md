---
phase: 02-orphan-detection-project-scope
plan: 01
subsystem: types, state
tags: typescript, json, atomic-write, filesystem

requires:
  - phase: 01-core-sync-pipeline
    provides: core types (Manifest, InstalledState, Diff, SyncOptions, SyncResult), normalizeSource in state.ts
provides:
  - Extended SyncOptions with yes, verbose, agent fields
  - Extended SyncResult with removed count
  - SourcesFile interface for .skills-sync-sources.json schema
  - sources.ts module with readSourcesFile, writeSourcesFile, detectInstalledSkills
affects: [02-02, 02-03, 02-04]

tech-stack:
  added: []
  patterns: [atomic write via tmp+rename, graceful null return on corrupt/missing state files]

key-files:
  created: [src/sources.ts, test/sources.test.ts]
  modified: [src/types.ts, src/executor.ts]

key-decisions:
  - "SOURCES_FILENAME exported as constant for use by other modules"
  - "writeSourcesFile normalizes keys via normalizeSource from state.ts — single canonical normalizer"
  - "detectInstalledSkills returns deduplicated array using Set to handle skills in both .agents/skills and .claude/skills"
  - "readSourcesFile returns null (not throws) for all error cases — consistent with state.ts graceful fallback pattern"

patterns-established:
  - "Atomic JSON write: write to .tmp then rename() — prevents corrupt state on crash"
  - "Source key normalization at write time — readSourcesFile returns keys as-stored"

requirements-completed: [STATE-03, SYNC-05]

duration: 3min
completed: 2026-03-01
---

# Plan 02-01: Source Attribution Types + Module Summary

**Extended core types for Phase 2 flags and created sources.ts with atomic read/write of .skills-sync-sources.json and filesystem skill detection**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-01
- **Completed:** 2026-03-01
- **Tasks:** 1
- **Files modified:** 4

## Accomplishments
- SyncOptions extended with yes, verbose, agent (all optional — backward compatible)
- SyncResult extended with removed count
- SourcesFile interface for version-guarded JSON schema
- sources.ts: readSourcesFile gracefully returns null on missing/corrupt/version-mismatch
- sources.ts: writeSourcesFile uses atomic tmp+rename pattern with normalized keys
- sources.ts: detectInstalledSkills scans both .agents/skills and .claude/skills, deduplicates

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend types and create sources.ts with read/write/detect** - `0d40d3a` (test+feat)

## Files Created/Modified
- `src/types.ts` - Extended SyncOptions, SyncResult, added SourcesFile interface
- `src/sources.ts` - New module: readSourcesFile, writeSourcesFile, detectInstalledSkills
- `test/sources.test.ts` - 15 unit tests covering all read/write/detect scenarios
- `src/executor.ts` - Added removed: 0 to SyncResult return (type compliance)

## Decisions Made
- SOURCES_FILENAME exported as public constant for use by other modules
- normalizeSource imported from state.ts rather than duplicated — single canonical normalizer
- detectInstalledSkills uses Set for deduplication (skills in both directories)
- readSourcesFile returns null on all error cases (consistent with graceful fallback pattern)

## Deviations from Plan
None - plan executed exactly as written

## Issues Encountered
- TypeScript error: SyncResult.removed added as required field, needed to update executor.ts return value to include `removed: 0`

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- sources.ts ready for import by state.ts (readSourcesFile) and CLI entry point (writeSourcesFile, detectInstalledSkills)
- Extended SyncOptions ready for flag threading through all pipeline stages
- All 122 tests pass, typecheck clean

---
*Phase: 02-orphan-detection-project-scope*
*Completed: 2026-03-01*
