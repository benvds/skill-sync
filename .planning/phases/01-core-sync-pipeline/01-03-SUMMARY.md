---
phase: 01-core-sync-pipeline
plan: 03
subsystem: infra
tags: [node, fs, json, vitest, typescript]

# Dependency graph
requires:
  - phase: 01-core-sync-pipeline
    plan: 01
    provides: InstalledState, Manifest, Diff types from src/types.ts
provides:
  - src/state.ts: readGlobalState (lock file parser with version guard) and readLocalState (filesystem scanner)
  - src/diff.ts: computeDiff (pure diff function) and normalizeSource (exported utility)
  - test/state.test.ts: 10 state reader tests with tmp-dir isolation and fixture files
  - test/diff.test.ts: 14 diff computation tests covering all edge cases
  - test/fixtures/skill-lock.json: valid v3 lock file fixture (2 sources, 3 skills)
  - test/fixtures/skill-lock-v99.json: version mismatch fixture
  - test/fixtures/skill-lock-corrupt.json: corrupt JSON fixture
affects:
  - 01-04 (executor imports diff.ts to decide what to install)
  - 01-05 (pipeline wires state readers + diff)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - TDD commit sequence for each module (RED fixtures+tests, GREEN implementation)
    - normalizeSource used for comparison only — original strings preserved in Diff output
    - __local__ synthetic source key for filesystem-scanned skills (no source attribution until Phase 2)
    - EXPECTED_LOCK_VERSION constant guards against lock file format drift

key-files:
  created:
    - src/state.ts
    - src/diff.ts
    - test/state.test.ts
    - test/diff.test.ts
    - test/fixtures/skill-lock.json
    - test/fixtures/skill-lock-v99.json
    - test/fixtures/skill-lock-corrupt.json
  modified: []

key-decisions:
  - "EXPECTED_LOCK_VERSION = 3 in state.ts — version mismatch falls back to empty state with console.error warning"
  - "All locally detected skills grouped under __local__ source key until Phase 2 adds .skills-sync-sources.json"
  - "normalizeSource exported from both state.ts and diff.ts — identical implementations kept in sync"
  - "Original (non-normalized) source strings preserved in Diff.toAdd and Diff.unchanged — normalization is for comparison only"

patterns-established:
  - "Pattern: Lock file read-only access — NEVER write to ~/.agents/.skill-lock.json"
  - "Pattern: Graceful degradation — missing files and corrupt JSON return empty state, never throw to callers"
  - "Pattern: Pure diff function — computeDiff has no I/O imports, deterministic output from inputs alone"
  - "Pattern: Test fixtures in test/fixtures/ directory with descriptive names for each error scenario"

requirements-completed:
  - STATE-01
  - STATE-02
  - SYNC-01

# Metrics
duration: 3min
completed: 2026-03-01
---

# Phase 1 Plan 03: State Readers and Diff Computation Summary

**Lock file parser with EXPECTED_LOCK_VERSION=3 guard, filesystem scanner for local skills, and pure diff function — 24 tests across two modules covering all edge cases including version mismatch, corrupt JSON, case normalization, and trailing slash handling.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-01T11:39:23Z
- **Completed:** 2026-03-01T11:42:05Z
- **Tasks:** 2 (Task 1: state readers, Task 2: diff computation)
- **Files modified:** 7 created, 4 fixed (import extensions)

## Accomplishments
- readGlobalState reads ~/.agents/.skill-lock.json with EXPECTED_LOCK_VERSION=3 guard; falls back to empty state on missing file, corrupt JSON, or version mismatch (with console.error warning)
- readLocalState scans .agents/skills/ and .claude/skills/ subdirectories, grouping all skills under __local__ synthetic key
- computeDiff is a pure function (zero I/O) that compares manifest sources against installed state with case-insensitive, trailing-slash-agnostic normalization while preserving original source strings in output
- 24 tests passing across state.test.ts (10) and diff.test.ts (14); tsc --noEmit exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Failing state reader tests + fixtures** - `0d9b96d` (test)
2. **Task 1 GREEN: Implement state readers** - `a9256c9` (feat)
3. **Task 2 RED: Failing diff tests** - `34b46d1` (test)
4. **Task 2 GREEN: Implement diff computation** - `c71c15e` (feat)
5. **Auto-fix: Import extension correction** - `85448e3` (fix)

_Note: TDD tasks produce test (RED) + feat (GREEN) commits_

## Files Created/Modified
- `src/state.ts` - readGlobalState (lock file parser) and readLocalState (filesystem scanner) + normalizeSource export
- `src/diff.ts` - computeDiff (pure diff function) and normalizeSource export
- `test/state.test.ts` - 10 tests: bySource grouping, normalization, missing file, corrupt JSON, version mismatch, local filesystem scanning
- `test/diff.test.ts` - 14 tests: all sources missing/present/mixed, case-insensitive, trailing slash, empty manifest/state, toRemove always empty, original strings preserved
- `test/fixtures/skill-lock.json` - Valid v3 lock file (2 sources: acme-org/dev-skills with 2 skills, Owner/Security-Tools with 1)
- `test/fixtures/skill-lock-v99.json` - Version 99 lock file (triggers version mismatch warning path)
- `test/fixtures/skill-lock-corrupt.json` - `{this is not json` (triggers JSON parse error path)

## Decisions Made
- Used EXPECTED_LOCK_VERSION = 3 constant as per locked decision; console.error (not throw) on mismatch keeps pipeline running
- All local skills grouped under `__local__` key — upstream gap (#337) noted in RESEARCH.md means source attribution is impossible without Phase 2's .skills-sync-sources.json
- normalizeSource is exported from both modules with identical implementation; future refactor could extract to shared utils if drift emerges
- Diff output always uses original manifest strings (not normalized) so downstream executor receives the exact strings the user wrote in .skills.toml

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed .ts import extensions to .js for Node16 ESM compatibility**
- **Found during:** Task 2 (tsc --noEmit verification after GREEN phase)
- **Issue:** New test files and source files used `.ts` extensions in import paths, causing tsc error TS5097 (allowImportingTsExtensions not enabled). Existing test/errors.test.ts uses `.js` extensions following Node16 ESM convention.
- **Fix:** Changed all import extensions from `.ts` to `.js` in src/state.ts, src/diff.ts, test/state.test.ts, and test/diff.test.ts
- **Files modified:** src/state.ts, src/diff.ts, test/state.test.ts, test/diff.test.ts
- **Verification:** tsc --noEmit exits 0; all 24 tests still passing
- **Committed in:** 85448e3 (fix commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug)
**Impact on plan:** Required for TypeScript compilation correctness; no scope change.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- src/state.ts and src/diff.ts are ready to import in Plan 04 (executor) and Plan 05 (pipeline wiring)
- All 3 requirements met: STATE-01 (lock file with version guard), STATE-02 (local filesystem scan), SYNC-01 (pure diff with correct split)
- tsc --noEmit and vitest run both clean

---
*Phase: 01-core-sync-pipeline*
*Completed: 2026-03-01*

## Self-Check: PASSED

All files verified present:
- src/state.ts, src/diff.ts: FOUND
- test/state.test.ts, test/diff.test.ts: FOUND
- test/fixtures/skill-lock.json, skill-lock-v99.json, skill-lock-corrupt.json: FOUND

All commits verified:
- 0d9b96d (RED state tests): FOUND
- a9256c9 (GREEN state): FOUND
- 34b46d1 (RED diff tests): FOUND
- c71c15e (GREEN diff): FOUND
- 85448e3 (fix import extensions): FOUND
