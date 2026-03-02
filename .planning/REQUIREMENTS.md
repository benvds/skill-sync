# Requirements: skills-sync

**Defined:** 2026-02-28
**Core Value:** Declarative, zero-friction skill synchronization: run one command and your skills directory matches your manifest.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Manifest

- [ ] **MNFST-01**: User can define skill repos in a `.skills.toml` manifest using `owner/repo` format
- [ ] **MNFST-02**: Tool validates repo entries match `^[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+$` pattern and rejects duplicates
- [ ] **MNFST-03**: Tool discovers manifest via CWD `.skills.toml` (local), `~/.config/skills-sync/skills.toml` (global), or explicit `--config <path>`
- [ ] **MNFST-04**: User can generate a documented sample `.skills.toml` via `--init` flag

### Repository Detection

- [ ] **REPO-01**: Tool detects single-skill repos (SKILL.md at root) via GitHub Trees API
- [ ] **REPO-02**: Tool detects monorepo skills (subdirectories containing SKILL.md, one level deep) via GitHub Trees API
- [ ] **REPO-03**: Tool skips repos with no SKILL.md found and warns the user
- [ ] **REPO-04**: Tool handles GitHub API rate limits gracefully with clear error messages showing remaining quota

### Sync Lifecycle

- [x] **SYNC-01**: Tool installs missing skills via shallow git clone (single-skill repos)
- [x] **SYNC-02**: Tool installs missing monorepo skills via sparse checkout to extract subdirectories
- [ ] **SYNC-03**: Tool updates outdated skills by comparing local SHA (from metadata) against remote HEAD SHA
- [x] **SYNC-04**: Tool detects orphaned skills (installed but not in manifest) and prompts user for removal
- [ ] **SYNC-05**: Tool writes `.skills.lock` per installed skill with source, SHA, and timestamps
- [x] **SYNC-06**: Tool syncs to local target (`./.claude/skills/`) by default or global target (`~/.claude/skills/`) with `--global` flag
- [ ] **SYNC-07**: Sync is idempotent — re-running with no changes produces no side effects

### CLI Flags

- [x] **CLI-01**: `--dry-run` shows planned changes (install/update/remove) without executing
- [x] **CLI-02**: `--force` skips confirmation prompts for non-interactive/CI usage
- [ ] **CLI-03**: `--verbose` shows detailed per-skill output including SHA comparisons
- [x] **CLI-04**: `--help` displays usage information with all flags and options
- [x] **CLI-05**: `--version` displays current version
- [x] **CLI-06**: `--global` and `--local` are mutually exclusive; conflicting usage produces clear error

### Error Handling

- [ ] **ERR-01**: Missing manifest file produces error with hint to run `--init`
- [ ] **ERR-02**: Invalid TOML syntax produces error with line number and description
- [x] **ERR-03**: Inaccessible/missing repos are warned and skipped; remaining repos continue
- [x] **ERR-04**: Network failures mid-sync leave no corrupted state; re-running resolves partial syncs
- [ ] **ERR-05**: Tool exits with code 0 on success and non-zero on any error
- [ ] **ERR-06**: GitHub API requests include User-Agent header to avoid 403 responses

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Version Control

- **VER-01**: User can pin a skill repo to a specific branch or tag (`owner/repo@v1.2.0`)
- **VER-02**: Lock file (`.skills-lock.toml`) pins exact SHAs for reproducible installs

### Authentication

- **AUTH-01**: User can configure GitHub token for private repo access beyond existing git/gh credentials

### Multi-Platform

- **PLAT-01**: Support GitLab and Bitbucket as additional skill sources

### Advanced

- **ADV-01**: `--json` output flag for machine-readable output
- **ADV-02**: Shell completions for zsh, bash, and fish

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Skill dependency resolution | No established convention for skill dependencies; skills are standalone by design |
| Skill validation/linting | Validation rules are ecosystem-dependent and unstable; trust repo maintainers |
| Watch mode / auto-sync daemon | File watching is OS-specific, resource-intensive; `skills-sync` is fast enough to run manually |
| TUI / web dashboard | Sync tools are used infrequently; invest in excellent `--verbose` and `--dry-run` instead |
| Multi-agent target support | Purpose-built for SKILL.md convention; "do one thing well" |
| Deeply nested monorepo scanning | One-level-deep is fast and predictable; deep nesting is an anti-pattern |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| MNFST-01 | Phase 1 | Pending |
| MNFST-02 | Phase 1 | Pending |
| MNFST-03 | Phase 1 | Pending |
| MNFST-04 | Phase 1 | Pending |
| REPO-01 | Phase 2 | Pending |
| REPO-02 | Phase 2 | Pending |
| REPO-03 | Phase 2 | Pending |
| REPO-04 | Phase 2 | Pending |
| SYNC-01 | Phase 3 | Complete |
| SYNC-02 | Phase 3 | Complete |
| SYNC-03 | Phase 2 | Pending |
| SYNC-04 | Phase 3 | Complete |
| SYNC-05 | Phase 1 | Pending |
| SYNC-06 | Phase 3 | Complete |
| SYNC-07 | Phase 2 | Pending |
| CLI-01 | Phase 4 | Complete |
| CLI-02 | Phase 4 | Complete |
| CLI-03 | Phase 4 | Pending |
| CLI-04 | Phase 4 | Complete |
| CLI-05 | Phase 4 | Complete |
| CLI-06 | Phase 4 | Complete |
| ERR-01 | Phase 1 | Pending |
| ERR-02 | Phase 1 | Pending |
| ERR-03 | Phase 3 | Complete |
| ERR-04 | Phase 3 | Complete |
| ERR-05 | Phase 1 | Pending |
| ERR-06 | Phase 2 | Pending |

**Coverage:**
- v1 requirements: 27 total
- Mapped to phases: 27
- Unmapped: 0

---
*Requirements defined: 2026-02-28*
*Last updated: 2026-03-02 after 03-02 completion (SYNC-04, SYNC-06, ERR-03 marked complete)*
