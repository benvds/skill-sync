# Plan 02-02 Summary: Sync resolver with skill detection

**Status:** Complete
**Duration:** ~5 min
**Commits:** 1

## What Was Built

Pure sync resolver module with:
- `detect_skills()`: Detects single-skill repos (SKILL.md at root) and monorepo skills (one-level-deep subdirectories with SKILL.md). Root SKILL.md always wins.
- `resolve()`: Compares remote skills against local metadata to produce Install/Update/Remove/Skip actions. Pure function, no I/O.
- `SyncAction` enum with Install, Update, Remove, Skip variants carrying repo, subdirectory, and SHA info
- `RemoteSkill` and `RepoSkills` types for structured remote state
- `skill_key()` helper for consistent lookup key generation

## Key Files

| File | Purpose |
|------|---------|
| src/resolver.rs | detect_skills, resolve, SyncAction, RemoteSkill, RepoSkills, skill_key |
| tests/resolver_tests.rs | 18 tests for detection and resolution logic |

## Test Results

18/18 tests passed covering: root skill detection, root-wins-over-subdirectory, monorepo detection, no-skills empty case, deep nesting ignored, tree/commit type ignored, install new, update changed SHA, skip unchanged, remove orphan, multiple mixed actions, idempotency, monorepo independence, same-repo-only orphan detection.

Total project: 69 tests passing.

## Deviations

- Removed unused `TreeEntry` import from resolver (only needs `TreeResponse` for the tree entries via its `tree` field).
- Fixed env var test race condition in github_tests.rs by adding a Mutex guard and save/restore pattern.

## Requirements Addressed

- REPO-01: Single-skill detection via root SKILL.md check
- REPO-02: Monorepo detection via one-level-deep SKILL.md scan
- REPO-03: Empty skills list when no SKILL.md found (detection layer)
- SYNC-03: Update action when SHA differs between local and remote
- SYNC-07: Skip action when SHA matches (idempotency)
