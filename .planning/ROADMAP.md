# Roadmap: skills-sync v2

## Overview

Three phases that build the declarative skill management CLI. Phase 1 delivers a working global-scope sync pipeline — the core value loop. Phase 2 completes the tool with orphan detection, project-scope tracking, and CI-ready flags. Phase 3 adds the CI gate (`--check`) and publishes to npm. Each phase ships something independently useful.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Core Sync Pipeline** - Parse manifest, read global state, diff, install, report
- [ ] **Phase 2: Orphan Detection + Project Scope** - Remove orphans, track local installs, CI-ready flags
- [ ] **Phase 3: CI Gate + Distribution** - `--check` exit code, npm publish

## Phase Details

### Phase 1: Core Sync Pipeline
**Goal**: Users can sync skills declaratively for global scope — commit `.skills.toml`, run `npx skills-sync`, installed skills match the manifest
**Depends on**: Nothing (first phase)
**Requirements**: MNFST-01, MNFST-02, MNFST-03, MNFST-04, STATE-01, STATE-02, SYNC-01, SYNC-02, FLAG-01, FLAG-02, OUTP-01, OUTP-03
**Success Criteria** (what must be TRUE):
  1. User can run `npx skills-sync --init` to generate a sample `.skills.toml` with comments
  2. User sees a clear error with line/column when `.skills.toml` has a syntax error
  3. User can run `npx skills-sync` and missing skills from the manifest get installed via `npx skills add`
  4. User can run `npx skills-sync --dry-run` to see what would be installed without executing
  5. User sees colored summary output: "X installed, Y unchanged" after sync completes
**Plans**: TBD

### Phase 2: Orphan Detection + Project Scope
**Goal**: Users get a complete sync — orphaned skills are detected and removed, local-scope installs are tracked, and CI pipelines can run non-interactively
**Depends on**: Phase 1
**Requirements**: STATE-03, SYNC-03, SYNC-04, SYNC-05, FLAG-03, FLAG-04, FLAG-05, OUTP-02
**Success Criteria** (what must be TRUE):
  1. User sees orphaned skills (installed but not in manifest) listed before sync, with a prompt to remove them
  2. User can run `npx skills-sync --yes` to skip all prompts and run non-interactively in CI
  3. User can run `npx skills-sync --verbose` to see the exact `npx skills` commands being executed
  4. User can pass `--agent <AGENT>` to target a specific agent for all installs
  5. After a local-scope sync, `.skills-sync-sources.json` reflects the installed sources so orphan detection works on the next run
**Plans**: TBD

### Phase 3: CI Gate + Distribution
**Goal**: Teams can enforce "manifest matches reality" as a CI gate, and anyone can run the tool via `npx skills-sync` without installing it globally
**Depends on**: Phase 2
**Requirements**: FLAG-06, DIST-01
**Success Criteria** (what must be TRUE):
  1. User can run `npx skills-sync --check` and it exits with code 1 when local state is out of sync with the manifest (including orphaned skills)
  2. User can run `npx skills-sync --check` and it exits with code 0 when state is fully in sync
  3. Any user can run `npx skills-sync` without prior installation via `npx`
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Core Sync Pipeline | 0/TBD | Not started | - |
| 2. Orphan Detection + Project Scope | 0/TBD | Not started | - |
| 3. CI Gate + Distribution | 0/TBD | Not started | - |
