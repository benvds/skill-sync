# Phase 1: Core Sync Pipeline - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the end-to-end sync pipeline: parse `.skills.toml` manifest, read installed state from Vercel's lock file and filesystem, compute diff (missing/present), install missing skills via `npx skills add`, and report results. Includes `--init`, `--dry-run`, `--global`/`--local` scope, `--config`, and colored summary output. Orphan detection/removal, `--yes`, `--verbose`, `--agent`, and `--check` are Phase 2-3.

</domain>

<decisions>
## Implementation Decisions

### Output format and reporting
- Default output is colored terminal text (not JSON)
- Summary line always printed: "X installed, Y unchanged" (or "X to install, Y unchanged" for dry-run)
- Use status symbols: ✓ (present/done), ✗ (error/skipped), ↓ (installing), ○ (pending)
- Errors include remediation hints: missing manifest → `--init` hint, invalid TOML → line/column, `npx skills` unavailable → install hint
- No spinner/progress bar for v1 — keep it simple, just print each action as it happens

### Flag design
- `--global` and `--local` are mutually exclusive; default is `--local`
- `--dry-run` prints planned actions and exits without executing
- `--init` creates sample `.skills.toml` in CWD (or `~/.config/skills-sync/skills.toml` with `--global`)
- `--config <PATH>` overrides manifest discovery
- `-h`/`--help` and `-V`/`--version` standard flags via commander

### Manifest discovery order
1. Explicit `--config <path>` flag
2. `.skills.toml` in CWD (for `--local`, which is default)
3. `~/.config/skills-sync/skills.toml` (for `--global`)
4. Error with hint to run `npx skills-sync --init`

### Lock file handling
- Read `~/.agents/.skill-lock.json` for global scope — never write to it
- Implement `EXPECTED_LOCK_VERSION` check from day one; warn and fall back gracefully on mismatch
- For local scope, scan `.agents/skills/` and `.claude/skills/` directories for installed skill directories
- If lock file is missing/unreadable, treat as empty state (everything needs installing)

### Subprocess execution
- Always pass `--yes` to `npx skills add` (even though Phase 1 doesn't expose `--yes` flag to user yet)
- Always pass `--global` flag when in global scope
- Use execa for subprocess — explicit stderr passthrough, no shell injection risk
- If `npx skills add` fails for one source, warn and continue with remaining sources
- Show the exact command being run (this becomes `--verbose` only in Phase 2, but implement the plumbing now)

### Source validation
- Sources in manifest are passed through as-is to `npx skills add` — no strict `owner/repo` format validation
- Validate: non-empty string, no duplicates in manifest
- Let `npx skills` handle format validation (GitHub, GitLab, local paths, URLs all supported)

### Claude's Discretion
- Exact project scaffold structure (src/ layout, tsconfig options)
- Commander program setup and help text formatting
- Internal error type design
- Test fixture design and test structure

</decisions>

<specifics>
## Specific Ideas

- Output should feel like npm install — familiar to JS developers: show each source being processed, then a summary line
- The `--init` template should be comprehensive with commented documentation (see PRD-v2.md for exact template)
- Entry point should support both `npx skills-sync` and direct `node` invocation via shebang (`#!/usr/bin/env node`)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- None directly reusable — v1 is Rust, v2 is a full TypeScript rewrite
- The `.skills.toml` format specification from v1 PRD is preserved and carries over

### Established Patterns
- No established TypeScript patterns yet — this is greenfield
- v1's module structure (manifest.rs, resolver.rs, init.rs) maps conceptually to v2's planned modules

### Integration Points
- `npx skills add <source> --yes [--global]` — the execution engine
- `~/.agents/.skill-lock.json` — Vercel's lock file (read-only)
- `.agents/skills/` and `.claude/skills/` — filesystem scan for local scope

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-core-sync-pipeline*
*Context gathered: 2026-03-01*
