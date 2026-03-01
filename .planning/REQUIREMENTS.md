# Requirements: skills-sync v2

**Defined:** 2026-03-01
**Core Value:** Declarative, reproducible skill management: commit one file, run one command, every teammate has the same skills.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Manifest

- [x] **MNFST-01**: User can parse `.skills.toml` with clear error messages including line/column on syntax errors
- [x] **MNFST-02**: User can run `--init` to scaffold a sample `.skills.toml` with documented comments and usage instructions
- [x] **MNFST-03**: User can specify custom manifest path via `--config <PATH>`
- [x] **MNFST-04**: Tool discovers manifest in order: explicit `--config`, `.skills.toml` in CWD (for `--local`), `~/.config/skills-sync/skills.toml` (for `--global`)

### State Reading

- [x] **STATE-01**: Tool reads Vercel's `~/.agents/.skill-lock.json` for global-scope installed skills with schema version validation
- [x] **STATE-02**: Tool scans `.agents/skills/` and `.claude/skills/` filesystem for project-scope installed skills
- [x] **STATE-03**: Tool reads `.skills-sync-sources.json` for project-scope source attribution (orphan detection)

### Sync

- [x] **SYNC-01**: Tool computes diff: missing sources (in manifest, not installed), orphaned skills (installed, not in manifest), present (in both)
- [x] **SYNC-02**: Tool installs missing skills by shelling out to `npx skills add <source> --yes` with appropriate scope flags
- [x] **SYNC-03**: Tool detects orphaned skills and prompts user for removal (or auto-removes with `--yes`)
- [x] **SYNC-04**: Tool removes orphaned skills by shelling out to `npx skills remove <name> --yes`
- [x] **SYNC-05**: Tool writes `.skills-sync-sources.json` after successful project-scope installs for source attribution

### CLI Flags

- [x] **FLAG-01**: `--global` and `--local` flags are mutually exclusive; default is `--local`
- [x] **FLAG-02**: `--dry-run` shows planned changes without executing
- [x] **FLAG-03**: `--yes` skips confirmation prompts and passes `-y` to `npx skills` subcommands
- [x] **FLAG-04**: `--verbose` shows the exact `npx skills` commands being executed
- [x] **FLAG-05**: `--agent <AGENT>` passes `--agent` to `npx skills add` commands
- [x] **FLAG-06**: `--check` exits with code 1 if local state is out of sync with manifest (CI gate)

### Output

- [x] **OUTP-01**: Tool prints colored summary: X installed, Y removed, Z unchanged
- [x] **OUTP-02**: Tool shows pre-sync diff preview with confirmation prompt (unless `--yes`)
- [x] **OUTP-03**: Error messages include remediation hints (missing manifest → `--init`, invalid TOML → line/column, `npx skills` unavailable → install hint)

### Distribution

- [ ] **DIST-01**: Tool is distributed as npm package runnable via `npx skills-sync`

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Advanced Features

- **ADV-01**: Version/SHA pinning for manifest entries (blocked by `npx skills` not supporting it)
- **ADV-02**: Git hook integration examples (pre-commit `--check`)
- **ADV-03**: Parallel `npx skills add` execution for users with many sources

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Custom GitHub API client | v1's mistake; `npx skills` handles all GitHub interaction |
| Custom skill detection (SKILL.md traversal) | `npx skills` handles monorepo detection and subdirectory traversal |
| Writing to Vercel's lock file | Would create state corruption; we read it, never write it |
| Watch mode / auto-sync | Adds complexity, background processes; no established need |
| Skill authoring / scaffolding | `npx skills init` already exists |
| Interactive skill selection UI | `npx skills find` already covers this |
| Private repo authentication config | Beyond scope; flows through `npx skills` |
| Dependency resolution between skills | Skills don't have dependencies on each other |
| Multi-level manifest inheritance | Too complex for <1000 LOC target |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| MNFST-01 | Phase 1 | Complete |
| MNFST-02 | Phase 1 | Complete |
| MNFST-03 | Phase 1 | Complete |
| MNFST-04 | Phase 1 | Complete |
| STATE-01 | Phase 1 | Complete |
| STATE-02 | Phase 1 | Complete |
| STATE-03 | Phase 2 | Complete |
| SYNC-01 | Phase 1 | Complete |
| SYNC-02 | Phase 1 | Complete |
| SYNC-03 | Phase 2 | Complete |
| SYNC-04 | Phase 2 | Complete |
| SYNC-05 | Phase 2 | Complete |
| FLAG-01 | Phase 1 | Complete |
| FLAG-02 | Phase 1 | Complete |
| FLAG-03 | Phase 2 | Complete |
| FLAG-04 | Phase 2 | Complete |
| FLAG-05 | Phase 2 | Complete |
| FLAG-06 | Phase 3 | Complete |
| OUTP-01 | Phase 1 | Complete |
| OUTP-02 | Phase 2 | Complete |
| OUTP-03 | Phase 1 | Complete |
| DIST-01 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 22 total
- Mapped to phases: 22
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-01*
*Last updated: 2026-03-01 after initial definition*
