---
phase: 01-core-sync-pipeline
plan: 01
subsystem: infra
tags: [typescript, node, commander, smol-toml, execa, picocolors, tsup, vitest]

# Dependency graph
requires: []
provides:
  - TypeScript project scaffold with ESM, Node 20+, all runtime and dev deps
  - src/types.ts: Manifest, InstalledState, Diff, SyncOptions, SyncResult interfaces
  - src/errors.ts: SkillsSyncError abstract base + 4 concrete typed error classes with hint strings
affects:
  - 01-02 (manifest parser imports types.ts and errors.ts)
  - 01-03 (state reader imports types.ts and errors.ts)
  - 01-04 (diff computation imports types.ts)
  - 01-05 (executor imports types.ts and errors.ts)
  - all subsequent plans in phase 01

# Tech tracking
tech-stack:
  added:
    - typescript 5.x (strict, Node16 module, ES2022 target)
    - tsup 8.x (ESM bundle, shebang banner, node20 target)
    - vitest 4.x (native TypeScript test runner)
    - commander 14.x (CLI argument parsing)
    - smol-toml 1.6.x (TOML 1.1.0 parsing with line/column errors)
    - execa 9.6.x (subprocess invocation, array API, stderr passthrough)
    - picocolors 1.1.x (terminal colors, NO_COLOR aware)
  patterns:
    - TDD with vitest (RED then GREEN commits)
    - Abstract base error class with concrete subclasses carrying hint strings
    - ESM-first TypeScript project (type: "module", Node16 module resolution)
    - Object.setPrototypeOf in error constructors for reliable instanceof checks

key-files:
  created:
    - package.json
    - tsconfig.json
    - tsup.config.ts
    - vitest.config.ts
    - bin/skills-sync.ts (placeholder entry point)
    - src/types.ts
    - src/errors.ts
    - test/errors.test.ts
  modified:
    - .gitignore (added dist/, node_modules/)
    - src/ (removed all Rust v1 .rs files)

key-decisions:
  - "ESM-only project (type: module) with Node16 module resolution — aligns with execa 9.x and modern Node.js"
  - "Object.setPrototypeOf in SkillsSyncError constructor ensures reliable instanceof checks across transpiled ESM"
  - "Removed Rust v1 source files from src/ — v2 is a full TypeScript rewrite"
  - "All error classes carry a hint string shown to users below the error message (OUTP-03)"

patterns-established:
  - "Pattern: Abstract error base with concrete typed subclasses — never throw raw Error for user-facing errors"
  - "Pattern: TDD commit sequence — test (RED) commit then feat (GREEN) commit per feature"
  - "Pattern: Explicit Object.setPrototypeOf in error constructors for instanceof safety in ESM"

requirements-completed:
  - OUTP-03

# Metrics
duration: 3min
completed: 2026-03-01
---

# Phase 1 Plan 01: Project Scaffold and Core Contracts Summary

**TypeScript ESM project scaffolded with commander/smol-toml/execa/picocolors stack; typed pipeline interfaces and hint-bearing error classes established as foundation for all subsequent plans.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-01T11:34:41Z
- **Completed:** 2026-03-01T11:36:59Z
- **Tasks:** 2 (Task 1: scaffold, Task 2: types + errors via TDD)
- **Files modified:** 10 created, 2 modified

## Accomplishments
- Removed Rust v1 source files and initialized clean TypeScript ESM project
- Installed full runtime and dev dependency stack (commander, smol-toml, execa, picocolors, typescript, tsup, vitest)
- Created tsconfig.json (Node16/ES2022/strict), tsup.config.ts (ESM + shebang), vitest.config.ts
- Defined all 5 shared interfaces in src/types.ts: Manifest, InstalledState, Diff, SyncOptions, SyncResult
- Implemented 4 typed error classes with user-facing remediation hints (OUTP-03 requirement)
- All 22 error tests pass; tsc --noEmit exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold TypeScript project** - `10c5f22` (chore)
2. **Task 2 RED: Failing error tests** - `d630e7e` (test)
3. **Task 2 GREEN: Implement types and errors** - `65c0c25` (feat)

_Note: TDD task produces test (RED) + feat (GREEN) commits_

## Files Created/Modified
- `package.json` - ESM npm project with skills-sync bin, node>=20, all deps declared
- `tsconfig.json` - TypeScript config (Node16 module, ES2022 target, strict mode)
- `tsup.config.ts` - Build config (ESM output, node20, shebang banner for bin)
- `vitest.config.ts` - Test runner config (test/**/*.test.ts pattern)
- `bin/skills-sync.ts` - Placeholder entry point (to be implemented in later plan)
- `src/types.ts` - Exports Manifest, InstalledState, Diff, SyncOptions, SyncResult
- `src/errors.ts` - Exports SkillsSyncError (abstract) + 4 concrete error classes with hints
- `test/errors.test.ts` - 22 tests covering instanceof hierarchy, message content, hint strings
- `.gitignore` - Added dist/, node_modules/ entries

## Decisions Made
- Used `Object.setPrototypeOf(this, new.target.prototype)` in SkillsSyncError constructor to ensure reliable instanceof checks across ESM transpilation boundaries
- Kept bin/skills-sync.ts as a minimal placeholder (just a comment) so tsconfig include pattern resolves without errors — full implementation is a later plan
- ManifestValidationError added per plan spec (covers malformed manifest structure beyond TOML parse errors)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- src/types.ts and src/errors.ts are ready to import in all subsequent plans
- Plan 01-02 (manifest parser) can begin: imports Manifest, ManifestNotFoundError, ManifestParseError, ManifestValidationError
- Plan 01-03 (state reader) can begin: imports InstalledState, SkillsUnavailableError
- tsc --noEmit and vitest run both clean — project foundation is solid

---
*Phase: 01-core-sync-pipeline*
*Completed: 2026-03-01*

## Self-Check: PASSED

All files verified present:
- package.json, tsconfig.json, tsup.config.ts, vitest.config.ts: FOUND
- src/types.ts, src/errors.ts: FOUND
- test/errors.test.ts, bin/skills-sync.ts: FOUND

All commits verified:
- 10c5f22 (scaffold): FOUND
- d630e7e (test RED): FOUND
- 65c0c25 (feat GREEN): FOUND
