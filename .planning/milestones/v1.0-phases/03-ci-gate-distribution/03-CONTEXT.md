# Phase 3: CI Gate + Distribution - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Add `--check` flag for CI gate enforcement (exits 0/1 based on sync state) and prepare the package for npm distribution via `npx skills-sync`. No new sync behavior — this phase makes the existing tool CI-ready and publicly installable.

</domain>

<decisions>
## Implementation Decisions

### --check output
- When out of sync: show full diff preview (reuse existing `printPreSyncPreview` — "To install:" and "To remove:" lists), then exit 1
- When in sync: print short confirmation like "✓ Skills in sync", then exit 0
- Only show problems (missing/orphaned) — do not list unchanged skills in check mode
- Include remediation hint when out of sync: "Run `npx skills-sync` to fix"

### Flag interactions
- `--check` + `--dry-run`: mutually exclusive — error out with clear message (both are read-only, redundant)
- `--check` respects `--global`/`--local` scope flags — same discovery logic as normal sync
- `--check` supports `--config` for custom manifest path — same discovery as sync
- `--check` does NOT support `--verbose` — check mode is purely diagnostic, no command output

### npm publish prep
- License: MIT
- Include a minimal README: what it does, install, usage example, flag reference
- Unscoped package name: `skills-sync` (cleaner `npx skills-sync` invocation)
- Add repository field pointing to GitHub repo (if git remote exists)

### Claude's Discretion
- Exact README structure and wording
- `files` field in package.json (what to include/exclude from published package)
- Whether to add `.npmignore` or rely on `files` field
- `publishConfig` settings
- tsup build configuration adjustments for distribution

</decisions>

<specifics>
## Specific Ideas

- Check output should match existing reporter patterns (same symbols, same color scheme)
- Remediation hint follows existing `reporter.printError` pattern (message + hint on separate lines)
- `--check` early-exits before any confirmation prompts or subprocess execution — pure read-only operation

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `computeDiff()` in `src/diff.ts`: Already returns `{ toAdd, toRemove, unchanged }` — `--check` just inspects this result
- `reporter.printPreSyncPreview()`: Shows "To install:" and "To remove:" lists — reuse for check output
- `reporter.printError()`: Error + hint pattern — reuse for remediation message
- `SyncOptions` type in `src/types.ts`: Add `check?: boolean` field

### Established Patterns
- CLI uses `commander` with `.option()` for flags — add `--check` the same way
- Mutually exclusive flags error via `reporter.printError()` + `process.exit(1)` (see `--global`/`--local`)
- Build via `tsup`, entry point in `bin/skills-sync.ts`, `"bin"` field already set in `package.json`

### Integration Points
- `bin/skills-sync.ts`: Insert `--check` handling after diff computation, before confirmation prompts
- `package.json`: Add `repository`, `license`, `files`, `keywords` for npm distribution
- `tsup.config.ts` (if exists) or `tsup` field in package.json for build config

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-ci-gate-distribution*
*Context gathered: 2026-03-01*
