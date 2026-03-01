# skills-sync v2

## What This Is

A thin TypeScript CLI that reads a declarative `.skills.toml` manifest and delegates to Vercel's `npx skills` package for installation, detection, and multi-agent logic. It turns imperative, one-at-a-time skill management into a declarative, reproducible workflow: commit one file, run one command, every teammate has the same skills.

## Core Value

**Declarative, reproducible skill management: commit one file, run one command, every teammate has the same skills.**

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] TOML manifest parsing and validation (`.skills.toml`)
- [ ] `--init` flag to generate sample manifest
- [ ] Read actual state from Vercel's lock file (`~/.agents/.skill-lock.json`) and filesystem
- [ ] Compute diff: missing sources to install, orphaned skills to remove, present skills to skip
- [ ] Shell out to `npx skills add/remove` for execution
- [ ] `--global` / `--local` scope flags (mutually exclusive, default `--local`)
- [ ] `--dry-run` flag for previewing changes
- [ ] `--yes` flag for CI (skip confirmation prompts)
- [ ] `--verbose` flag for detailed output
- [ ] `--agent` flag passthrough to `npx skills add`
- [ ] `--config <PATH>` custom manifest path
- [ ] Orphan detection and removal workflow
- [ ] `.skills-sync-sources.json` for project-scope source attribution
- [ ] Colored CLI output and summary reporting
- [ ] `--check` flag (exit code 1 if out of sync — CI gates)
- [ ] npm package distribution via `npx skills-sync`

### Out of Scope

- Reimplementing skill detection, GitHub API, or git operations — delegate to `npx skills`
- Version pinning — `skills` doesn't support it either
- Skill authoring tools — use `npx skills init`
- Competing with `npx skills` — complement it
- Rust implementation — superseded by TypeScript pivot
- Private repo authentication beyond what `skills` provides
- Watch mode or auto-sync

## Context

- **Strategic pivot:** Originally a Rust CLI that reimplemented GitHub API interaction, skill detection, and git operations. Vercel's `skills` npm package (`npx skills`) has since become the dominant ecosystem tool. skills-sync's unique value is the declarative manifest — not the plumbing.
- **Existing codebase:** There is existing Rust code from the v1 implementation (phases 1-2 completed). The v2 is a full rewrite in TypeScript.
- **`.skills.toml` format:** Preserved from v1 — this is the core IP. Format uses `[skills]` section with `repos` array of source strings.
- **Ecosystem:** Vercel's `skills` supports 40+ agents, multi-source (GitHub, GitLab, local paths, URLs), and manages `~/.agents/.skill-lock.json` for global installs.
- **Target size:** ~500 lines of orchestration logic in TypeScript.

## Constraints

- **Tech stack**: TypeScript, distributed as npm package via `npx skills-sync`
- **Dependency**: Must work with Vercel's `npx skills` package as execution engine
- **Lock file**: Read but never write Vercel's `~/.agents/.skill-lock.json` — we're a layer on top
- **Compatibility**: `.skills.toml` format must be 100% backward-compatible with v1

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| TypeScript over Rust | CLI is ~500 LOC orchestration; Rust overkill for thin wrapper; npm distribution aligns with `skills` ecosystem | — Pending |
| Delegate to `npx skills` | Avoids reimplementing GitHub API, skill detection, git ops; leverages 40+ agent support | — Pending |
| Read Vercel's lock file directly | Faster than `npx skills list`; lock file format is stable at v3 | — Pending |
| Source-level tracking (not skill-level) | A source may produce multiple skills (monorepo); track at source granularity | — Pending |
| `.skills-sync-sources.json` for project scope | Bridges gap where Vercel's lock file doesn't cover project-scoped installs | — Pending |

---
*Last updated: 2026-03-01 after initialization*
