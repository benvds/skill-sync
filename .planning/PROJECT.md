# skills-sync v2

## What This Is

A TypeScript CLI that reads a declarative `.skills.toml` manifest and delegates to Vercel's `npx skills` package for installation, detection, and multi-agent logic. Commit one file, run one command, every teammate has the same skills. Includes orphan detection, CI gate (`--check`), and npm distribution.

## Core Value

**Declarative, reproducible skill management: commit one file, run one command, every teammate has the same skills.**

## Requirements

### Validated

- ✓ TOML manifest parsing and validation (`.skills.toml`) — v1.0
- ✓ `--init` flag to generate sample manifest — v1.0
- ✓ Read actual state from Vercel's lock file and filesystem — v1.0
- ✓ Compute diff: missing sources to install, orphaned skills to remove, present skills to skip — v1.0
- ✓ Shell out to `npx skills add/remove` for execution — v1.0
- ✓ `--global` / `--local` scope flags (mutually exclusive, default `--local`) — v1.0
- ✓ `--dry-run` flag for previewing changes — v1.0
- ✓ `--yes` flag for CI (skip confirmation prompts) — v1.0
- ✓ `--verbose` flag for detailed output — v1.0
- ✓ `--agent` flag passthrough to `npx skills add` — v1.0
- ✓ `--config <PATH>` custom manifest path — v1.0
- ✓ Orphan detection and removal workflow — v1.0
- ✓ `.skills-sync-sources.json` for project-scope source attribution — v1.0
- ✓ Colored CLI output and summary reporting — v1.0
- ✓ `--check` flag (exit code 1 if out of sync — CI gates) — v1.0
- ✓ npm package distribution via `npx skills-sync` — v1.0

### Active

(None — planning next milestone)

### Out of Scope

- Reimplementing skill detection, GitHub API, or git operations — delegate to `npx skills`
- Version pinning — `skills` doesn't support it either
- Skill authoring tools — use `npx skills init`
- Competing with `npx skills` — complement it
- Private repo authentication beyond what `skills` provides
- Watch mode or auto-sync
- Multi-level manifest inheritance — too complex for current scope

## Context

- **v1.0 shipped:** 2026-03-01. 2,703 LOC TypeScript across 3 phases, 10 plans.
- **Strategic pivot validated:** Thin wrapper over `npx skills` proved correct — 500 LOC target achieved.
- **Tech stack:** TypeScript, ESM-only, Node 20+. Dependencies: commander, smol-toml, execa, picocolors, @inquirer/prompts. Build: tsup. Test: vitest (156 passing).
- **`.skills.toml` format:** Preserved from v1. `[skills]` section with `repos` array of source strings.
- **Known tech debt:** 7 non-critical items (dead code, dry-run/orphan prompt interaction, init error typing). See MILESTONES.md.

## Constraints

- **Tech stack**: TypeScript, distributed as npm package via `npx skills-sync`
- **Dependency**: Must work with Vercel's `npx skills` package as execution engine
- **Lock file**: Read but never write Vercel's `~/.agents/.skill-lock.json` — we're a layer on top
- **Compatibility**: `.skills.toml` format must be 100% backward-compatible with v1

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| TypeScript over Rust | CLI is ~500 LOC orchestration; Rust overkill for thin wrapper; npm distribution aligns with `skills` ecosystem | ✓ Good — shipped 2,703 LOC in 2 days |
| Delegate to `npx skills` | Avoids reimplementing GitHub API, skill detection, git ops; leverages 40+ agent support | ✓ Good — zero GitHub/git code written |
| Read Vercel's lock file directly | Faster than `npx skills list`; lock file format is stable at v3 | ✓ Good — direct JSON parse, version-guarded |
| Source-level tracking (not skill-level) | A source may produce multiple skills (monorepo); track at source granularity | ✓ Good — orphan detection works correctly |
| `.skills-sync-sources.json` for project scope | Bridges gap where Vercel's lock file doesn't cover project-scoped installs | ✓ Good — enables local orphan detection |
| ESM-only with Node16 resolution | Aligns with execa 9.x and modern Node.js | ✓ Good — no CJS compatibility issues |
| `--dry-run` boundary in executor only | Single check point before execa call; no flag threading | ✓ Good — clean separation |
| Batch removal in single subprocess | All orphans removed in one `npx skills remove` call | ✓ Good — efficient |

---
*Last updated: 2026-03-01 after v1.0 milestone*
