# Roadmap: skills-sync

## Overview

Four phases deliver a working Rust CLI from scratch: Phase 1 lays the crate foundation with manifest parsing and metadata types; Phase 2 integrates the GitHub API and builds the pure sync resolver; Phase 3 executes the sync — git clones, updates, orphan removal, and network failure safety; Phase 4 wires everything through the full CLI surface with flags, UX polish, and distribution automation. Each phase delivers a verifiable capability, and correctness concerns are front-loaded so UX can be layered on a stable engine.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - Crate scaffold, manifest parsing, metadata types, and error infrastructure (completed 2026-02-28)
- [x] **Phase 2: GitHub + Resolver** - GitHub API integration and pure sync plan logic (completed 2026-02-28)
- [ ] **Phase 3: Sync Engine** - Git operations, install/update/orphan flow, and failure safety
- [ ] **Phase 4: CLI + UX** - Full CLI surface, flags, output polish, and distribution

## Phase Details

### Phase 1: Foundation
**Goal**: A compiling crate that can parse a `.skills.toml` manifest, validate it, discover it from multiple locations, and read/write per-skill metadata atomically
**Depends on**: Nothing (first phase)
**Requirements**: MNFST-01, MNFST-02, MNFST-03, MNFST-04, SYNC-05, ERR-01, ERR-02, ERR-05, ERR-06
**Success Criteria** (what must be TRUE):
  1. Running `skills-sync --init` produces a documented sample `.skills.toml` in the current directory
  2. Running `skills-sync` with a valid `.skills.toml` parses and validates all repo entries without panicking
  3. Running `skills-sync` with an invalid or missing manifest exits with a non-zero code and a human-readable error indicating what went wrong and how to fix it
  4. A per-skill `.skills-sync-meta.json` file written during a simulated install contains source, SHA, and timestamps and is written atomically (via rename)
  5. Tool exits with code 0 when no error occurs and non-zero on any failure
**Plans**: TBD

### Phase 2: GitHub + Resolver
**Goal**: The tool can call the GitHub Trees API to detect repo structure, fetch HEAD SHAs, and produce a correct sync plan describing what needs to be installed, updated, or removed
**Depends on**: Phase 1
**Requirements**: REPO-01, REPO-02, REPO-03, REPO-04, SYNC-03, SYNC-07, ERR-06
**Success Criteria** (what must be TRUE):
  1. Given a single-skill repo (SKILL.md at root), the tool correctly identifies it as a single-skill repo via the Trees API
  2. Given a monorepo (subdirectories with SKILL.md), the tool correctly identifies each skill subdirectory one level deep
  3. A repo with no SKILL.md anywhere produces a visible warning and is skipped without aborting the run
  4. Running sync twice with no upstream changes produces no planned actions (idempotency verified via resolver output)
  5. GitHub API rate limit errors produce a clear message showing remaining quota rather than a generic crash
**Plans**: 3 plans
- [x] 02-01-PLAN.md — GitHub API client with auth discovery and Trees API integration
- [x] 02-02-PLAN.md — Pure sync resolver with skill detection and action resolution
- [x] 02-03-PLAN.md — Wire GitHub client + resolver into main run() pipeline

### Phase 3: Sync Engine
**Goal**: The tool can install, update, and remove skills on disk, targeting local or global directories, while leaving no corrupted state on network failure or interruption
**Depends on**: Phase 2
**Requirements**: SYNC-01, SYNC-02, SYNC-04, SYNC-06, ERR-03, ERR-04
**Success Criteria** (what must be TRUE):
  1. A skill listed in the manifest that is absent on disk gets installed via shallow clone into `./.claude/skills/` (local) or `~/.claude/skills/` (global)
  2. A monorepo skill is installed via sparse checkout — only the relevant subdirectory is extracted to disk
  3. An orphaned skill (on disk but not in manifest) triggers a removal prompt and is removed upon confirmation
  4. A network failure mid-install leaves no corrupted skill directory — re-running the tool completes the install cleanly
  5. An inaccessible repo is warned and skipped; all remaining repos continue to sync
**Plans**: TBD

### Phase 4: CLI + UX
**Goal**: A fully usable CLI with all flags working, color-coded output, progress feedback, and binary distribution via Homebrew and GitHub Releases
**Depends on**: Phase 3
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06
**Success Criteria** (what must be TRUE):
  1. `--dry-run` prints every planned action (install/update/remove) without touching disk or network state
  2. `--force` runs to completion without any interactive prompts, suitable for CI environments
  3. `--verbose` shows per-skill SHA comparisons and git command output
  4. `--help` displays all flags and `--version` displays the current binary version
  5. Passing both `--global` and `--local` produces a clear mutual-exclusivity error before any network calls
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 0/TBD | Complete    | 2026-02-28 |
| 2. GitHub + Resolver | 3/3 | Complete | 2026-02-28 |
| 3. Sync Engine | 0/TBD | Not started | - |
| 4. CLI + UX | 0/TBD | Not started | - |
