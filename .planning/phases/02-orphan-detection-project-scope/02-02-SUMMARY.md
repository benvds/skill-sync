---
phase: 02-orphan-detection-project-scope
plan: 02
subsystem: state, diff
tags: typescript, orphan-detection, source-attribution

requires:
  - phase: 02-orphan-detection-project-scope
    provides: readSourcesFile from sources.ts, SourcesFile interface from types.ts
provides:
  - readLocalState with .skills-sync-sources.json support and __local__ fallback
  - computeDiff with toRemove populated from orphaned skill names
affects: [02-04]

tech-stack:
  added: []
  patterns: [graceful fallback from sources.json to filesystem scan, __local__ sentinel guard]

key-files:
  created: []
  modified: [src/state.ts, src/diff.ts, test/state.test.ts, test/diff.test.ts]

key-decisions:
  - "readLocalState checks readSourcesFile first, falls back to filesystem scan — zero behavior change when file absent"
  - "computeDiff guards orphan detection with __local__ check — prevents false orphan detection on first-ever run"
  - "toRemove contains skill directory NAMES (not source URLs) — matches npx skills remove argument format"
  - "Updated Phase 1 test 'returns empty result for empty manifest' to reflect new orphan detection behavior"

patterns-established:
  - "__local__ sentinel key as guard for orphan detection availability"

requirements-completed: [SYNC-03]

duration: 3min
completed: 2026-03-01
---

# Plan 02-02: State/Diff Source Attribution + Orphan Detection Summary

**readLocalState reads .skills-sync-sources.json for source attribution; computeDiff detects orphaned skill names via toRemove**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-01
- **Completed:** 2026-03-01
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- readLocalState checks .skills-sync-sources.json first, falls back to __local__ filesystem scan
- computeDiff populates toRemove with orphaned skill names from sources not in manifest
- __local__ sentinel guard prevents false orphan detection when no source attribution exists
- toRemove contains skill directory names (not source URLs) for npx skills remove

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend readLocalState** - `cfd518b` (feat)
2. **Task 2: Extend computeDiff toRemove** - `6c4bb74` (feat)

## Files Created/Modified
- `src/state.ts` - readLocalState reads .skills-sync-sources.json first via readSourcesFile
- `src/diff.ts` - computeDiff populates toRemove with orphaned skill names
- `test/state.test.ts` - 4 new tests for sources.json support and fallback
- `test/diff.test.ts` - 6 new tests for toRemove population and __local__ guard

## Decisions Made
- Updated Phase 1 test that asserted toRemove always empty — now asserts correct orphan detection behavior
- Consistent normalizeSource comparison used for source matching (case-insensitive, trailing-slash-agnostic)

## Deviations from Plan
None - plan executed exactly as written

## Issues Encountered
- Phase 1 test "returns empty result for empty manifest regardless of state" needed updating since toRemove is now populated with orphans

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- computeDiff returns complete Diff with toRemove ready for executor removal
- readLocalState source attribution enables orphan detection on subsequent runs

---
*Phase: 02-orphan-detection-project-scope*
*Completed: 2026-03-01*
