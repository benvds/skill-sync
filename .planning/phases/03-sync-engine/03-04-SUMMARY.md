---
phase: 03-sync-engine
plan: "04"
subsystem: manifest
tags: [rust, regex, parsing, monorepo, subdirectory, tdd]

requires:
  - phase: 03-sync-engine
    provides: sync engine, resolver, sync.rs skill_dir_name, main.rs per-repo loop

provides:
  - parse_repo_entry() helper in src/manifest.rs splitting owner/repo/optional-subdir
  - Updated REPO_PATTERN regex accepting owner/repo/path/to/skill format
  - Updated main.rs using parse_repo_entry with conditional Trees API detection bypass
  - Fixed skill_dir_name flattening multi-segment subdirectory slashes to --
  - Updated error messages and init template documenting subdir format

affects: [main, manifest, sync]

tech-stack:
  added: []
  patterns:
    - "parse_repo_entry splits manifest entries into (owner, repo, Option<subdir>) using splitn(3, '/')"
    - "manifest_repos and branch_map keyed by owner/repo slug, not full manifest entry"
    - "When manifest subdir specified, bypass detect_skills() and create RemoteSkill directly"
    - "skill_dir_name uses subdir.replace('/', '--') to produce flat filesystem paths"

key-files:
  created: []
  modified:
    - src/manifest.rs
    - src/main.rs
    - src/sync.rs
    - src/error.rs
    - src/init.rs
    - tests/manifest_tests.rs
    - tests/sync_tests.rs

key-decisions:
  - "First two path segments always treated as owner/repo regardless of total segment count"
  - "manifest_repos uses owner/repo slug (not full manifest path) so orphan detection matches meta.source"
  - "When manifest specifies subdir, Trees API detection is skipped entirely — monorepo layout detection only runs for bare owner/repo entries"
  - "skill_dir_name replaces slashes within subdirectory with -- to prevent nested directory creation"

patterns-established:
  - "parse_repo_entry: canonical entry point for splitting any manifest repo string"

requirements-completed: [SYNC-01, SYNC-02, SYNC-06]

duration: 3min
completed: "2026-03-02"
---

# Phase 3 Plan 4: Monorepo Subdirectory Manifest Support Summary

**Manifest parser extended to accept `owner/repo/path/to/skill` format, with parse_repo_entry() helper, conditional Trees API bypass, and flattened skill_dir_name for multi-segment subdirs**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-02T14:28:01Z
- **Completed:** 2026-03-02T14:31:12Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- Updated REPO_PATTERN regex to accept `owner/repo`, `owner/repo/sub`, `owner/repo/a/b/c` formats
- Added `parse_repo_entry()` public function splitting entries into `(owner, repo_name, Option<subdir>)`
- Fixed `skill_dir_name()` to flatten multi-segment subdirs: `skills/creator` becomes `skills--creator` (not `skills/creator` which would create nested dirs)
- Updated `main.rs` per-repo loop to use `parse_repo_entry`, extract `repo_slug`, and conditionally bypass Trees API detection when manifest specifies a subdirectory
- Updated error help text and init template to document the `owner/repo/subdir` format
- Added 14 new manifest tests and 1 new sync test; all 101 tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Update manifest parser to accept owner/repo/subdir format and add parse_repo_entry helper** - `d60c4e9` (feat, TDD)
2. **Task 2: Update main.rs, fix skill_dir_name for multi-segment subdirs, integrate parse_repo_entry** - `659ee22` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/manifest.rs` - Updated REPO_PATTERN regex, added parse_repo_entry() public helper
- `src/main.rs` - Uses parse_repo_entry, repo_slug for manifest_repos/branch_map, conditional Trees API bypass
- `src/sync.rs` - skill_dir_name now uses subdir.replace('/', "--") to flatten multi-segment paths
- `src/error.rs` - Updated InvalidRepo help text to mention owner/repo/subdir format
- `src/init.rs` - Updated init template with monorepo subdir documentation and examples
- `tests/manifest_tests.rs` - 14 new tests: parse_repo_entry, subdir validation, duplicate edge cases
- `tests/sync_tests.rs` - 1 new test: skill_dir_name_multi_segment_subdir

## Decisions Made
- First two path segments always treated as owner/repo — this is unambiguous given GitHub's URL structure
- `manifest_repos` keyed by `owner/repo` slug (not full manifest path) so orphan detection correctly matches `meta.source` which stores the slug
- When manifest specifies an explicit subdirectory, Trees API skill detection is bypassed entirely — the specified subdir is used directly for the SyncAction
- `skill_dir_name` replaces slashes within subdirectory with `--` to prevent nested directory creation on the filesystem

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Monorepo skill entries like `anthropics/skills/skills/skill-creator` now work end-to-end
- Manifest parser, main.rs integration, and sync engine all consistent with owner/repo slug as the canonical identifier
- Plan 05 (gap closure) can proceed
- All 101 existing tests continue to pass

---
*Phase: 03-sync-engine*
*Completed: 2026-03-02*
