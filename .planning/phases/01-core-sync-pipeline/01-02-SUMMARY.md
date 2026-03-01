---
phase: 01-core-sync-pipeline
plan: 02
subsystem: manifest
tags: [typescript, smol-toml, toml, manifest, init, config-discovery]

# Dependency graph
requires:
  - phase: 01-01
    provides: src/types.ts (Manifest interface), src/errors.ts (ManifestNotFoundError, ManifestParseError, ManifestValidationError)
provides:
  - src/manifest.ts: resolveManifestPath, readManifest, parseManifest with full validation
  - src/init.ts: generateManifestTemplate, writeManifestTemplate
  - test/manifest.test.ts: 17 tests covering parsing, validation, config discovery
  - test/init.test.ts: 14 tests covering template generation and file writing
  - test/fixtures: valid, invalid, empty-repos, duplicate-repos TOML fixtures
affects:
  - 01-05 (executor wires readManifest into sync pipeline)
  - bin/skills-sync.ts (calls readManifest and writeManifestTemplate from CLI)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - smol-toml parse() with TomlError catch for line/column errors
    - Exported parseManifest for direct unit testing (internal function made testable)
    - resolveManifestPath: locked priority --config > local CWD > global ~/.config
    - writeManifestTemplate: refuses overwrite via fs.access guard

key-files:
  created:
    - src/manifest.ts
    - src/init.ts
    - test/manifest.test.ts
    - test/init.test.ts
    - test/fixtures/valid.skills.toml
    - test/fixtures/invalid.skills.toml
    - test/fixtures/empty-repos.skills.toml
    - test/fixtures/duplicate-repos.skills.toml
  modified: []

key-decisions:
  - "parseManifest exported as a public function to allow direct unit testing without filesystem I/O"
  - "Sources trimmed but NOT normalized in manifest.ts — case normalization is diff.ts responsibility"
  - "Duplicate detection is case-sensitive at manifest level (owner/repo vs Owner/Repo are different manifest entries)"
  - "writeManifestTemplate uses fs.access guard before write — throws with descriptive error on existing file"

patterns-established:
  - "Pattern: Export internal parse function for unit testability alongside the full readX pipeline function"
  - "Pattern: macOS symlink fix in tests — use fs.realpath on mkdtemp result so process.cwd() matches expected paths"

requirements-completed:
  - MNFST-01
  - MNFST-02
  - MNFST-03
  - MNFST-04

# Metrics
duration: 2min
completed: 2026-03-01
---

# Phase 1 Plan 02: Manifest Parsing Pipeline Summary

**TOML manifest reader with smol-toml line/column errors, locked config discovery order, and --init template generator; 31 tests across manifest and init modules.**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-01T11:39:17Z
- **Completed:** 2026-03-01T11:41:32Z
- **Tasks:** 2 (Task 1: manifest parser via TDD, Task 2: init template via TDD)
- **Files modified:** 8 created

## Accomplishments
- Implemented `resolveManifestPath` with locked priority: `--config` > CWD `.skills.toml` (local) > `~/.config/skills-sync/skills.toml` (global)
- Implemented `readManifest` + `parseManifest` with full validation: `[skills]` table, `repos` array, non-empty strings, no duplicates
- `parseManifest` catches smol-toml `TomlError` to preserve line/column location in `ManifestParseError`
- `generateManifestTemplate` returns comprehensive template with GitHub shorthand, full URL, local path, and usage instruction examples
- `writeManifestTemplate` creates parent dirs for global scope, refuses to overwrite existing file
- 31 tests all passing; vitest run exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Failing manifest tests** - `799e665` (test)
2. **Task 1 GREEN: Implement manifest parsing** - `e4bf0b6` (feat)
3. **Task 2 RED: Failing init tests** - `b6c2263` (test)
4. **Task 2 GREEN: Implement init template** - `afd01c0` (feat)

_Note: TDD tasks produce test (RED) + feat (GREEN) commits_

## Files Created/Modified
- `src/manifest.ts` - resolveManifestPath, readManifest, parseManifest, validateManifest (142 lines)
- `src/init.ts` - generateManifestTemplate, writeManifestTemplate (77 lines)
- `test/manifest.test.ts` - 17 tests: parsing, validation, config discovery, readManifest pipeline
- `test/init.test.ts` - 14 tests: template content, writeManifestTemplate file creation and overwrite guard
- `test/fixtures/valid.skills.toml` - Two-source valid manifest
- `test/fixtures/invalid.skills.toml` - Deliberately broken TOML (unclosed bracket) for parse error tests
- `test/fixtures/empty-repos.skills.toml` - Empty repos array
- `test/fixtures/duplicate-repos.skills.toml` - Duplicate source entries

## Decisions Made
- `parseManifest` exported as a public function to enable direct unit testing without filesystem I/O
- Sources are trimmed (`entry.trim()`) but NOT lowercased/normalized in manifest.ts — case normalization is `diff.ts` responsibility per plan spec
- Duplicate detection in manifest is case-sensitive: `"owner/repo"` and `"Owner/Repo"` would be accepted as distinct entries (normalization happens in diff)
- `writeManifestTemplate` checks file existence via `fs.access` before writing; throws descriptive error mentioning file path on overwrite attempt

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed macOS symlink path mismatch in init tests**
- **Found during:** Task 2 GREEN (init tests initially failed with 2 failures)
- **Issue:** `os.mkdtemp()` returns `/var/folders/...` but `process.cwd()` after `chdir` returns `/private/var/folders/...` (macOS resolves symlinks in cwd). Test `expect(writtenPath).toBe(path.join(tempDir, '.skills.toml'))` failed because `tempDir` was the symlinked path.
- **Fix:** Added `tempDir = await fs.realpath(rawTempDir)` in `beforeEach` so `tempDir` is the real (non-symlinked) path matching `process.cwd()`
- **Files modified:** test/init.test.ts
- **Verification:** All 14 init tests pass
- **Committed in:** afd01c0 (Task 2 GREEN commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug)
**Impact on plan:** Necessary correctness fix for test reliability on macOS. No scope creep.

## Issues Encountered

- Pre-existing `tsc --noEmit` errors in `test/diff.test.ts` and `test/state.test.ts` (using `.ts` extension in imports instead of `.js`). These files predate this plan and are out of scope. Logged to deferred items. All plan-relevant files type-check cleanly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `src/manifest.ts` is ready for wiring in `bin/skills-sync.ts`
- All 4 manifest requirements met: MNFST-01 (line/column errors), MNFST-02 (--init template), MNFST-03 (--config honored), MNFST-04 (discovery order)
- `src/init.ts` ready for `--init` CLI wiring
- Plan 01-03 (state reader) can proceed; it imports from `src/types.ts` and `src/errors.ts` only

---
*Phase: 01-core-sync-pipeline*
*Completed: 2026-03-01*

## Self-Check: PASSED

All files verified present:
- src/manifest.ts: FOUND
- src/init.ts: FOUND
- test/manifest.test.ts: FOUND
- test/init.test.ts: FOUND
- test/fixtures/valid.skills.toml: FOUND
- test/fixtures/invalid.skills.toml: FOUND
- test/fixtures/empty-repos.skills.toml: FOUND
- test/fixtures/duplicate-repos.skills.toml: FOUND
- .planning/phases/01-core-sync-pipeline/01-02-SUMMARY.md: FOUND

All commits verified:
- 799e665 (test RED - manifest): FOUND
- e4bf0b6 (feat GREEN - manifest): FOUND
- b6c2263 (test RED - init): FOUND
- afd01c0 (feat GREEN - init): FOUND
- 4474855 (docs metadata): FOUND
