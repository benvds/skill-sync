# Phase 3: CI Gate + Distribution - Research

**Researched:** 2026-03-01
**Domain:** CLI flag implementation + npm package distribution
**Confidence:** HIGH

## Summary

Phase 3 has two distinct deliverables: (1) a `--check` flag that exits 0/1 based on sync state for CI gate enforcement, and (2) preparing `package.json` for npm distribution so users can run `npx skills-sync` without prior installation.

Both are low-risk additions to an already-complete codebase. The `--check` flag reuses `computeDiff()` and `reporter.printPreSyncPreview()` — no new modules needed. npm distribution requires metadata fields in `package.json` and a `files` whitelist.

**Primary recommendation:** Implement `--check` as an early-exit path in `bin/skills-sync.ts` that runs the diff pipeline but never reaches the executor. npm prep is pure `package.json` + README work with zero code changes.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **--check output:** When out of sync: show full diff preview (reuse `printPreSyncPreview`), then exit 1. When in sync: print short confirmation like "Skills in sync", exit 0. Only show problems. Include remediation hint when out of sync.
- **Flag interactions:** `--check` + `--dry-run` mutually exclusive (error out). `--check` respects `--global`/`--local` scope flags. `--check` supports `--config`. `--check` does NOT support `--verbose`.
- **npm publish prep:** License MIT. Include minimal README. Unscoped package name `skills-sync`. Add repository field.

### Claude's Discretion
- Exact README structure and wording
- `files` field in package.json
- Whether to add `.npmignore` or rely on `files` field
- `publishConfig` settings
- tsup build configuration adjustments for distribution

### Deferred Ideas (OUT OF SCOPE)
- None
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FLAG-06 | `--check` exits with code 1 if local state is out of sync with manifest (CI gate) | Reuse `computeDiff()` result — check `toAdd.length > 0 \|\| toRemove.length > 0` for exit 1. Early-exit in CLI before any executor/prompt logic. |
| DIST-01 | Tool is distributed as npm package runnable via `npx skills-sync` | Add `files`, `license`, `repository`, `keywords` to `package.json`. `bin` field already configured. tsup already builds to `dist/`. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| commander | ^14.0.3 | CLI flag parsing | Already used — add `--check` with `.option()` |
| picocolors | ^1.1.1 | Colored output | Already used — reporter patterns for check output |
| tsup | ^8.5.1 | Build/bundle | Already configured — builds ESM entry point with shebang |

### Supporting
No new libraries needed. All functionality is covered by existing dependencies.

### Alternatives Considered
None — phase uses exclusively existing infrastructure.

## Architecture Patterns

### --check Implementation Pattern

The `--check` flag follows the existing CLI early-exit pattern (like `--init`):

```typescript
// In bin/skills-sync.ts, after computeDiff() but before any prompts/execution:
if (options.check) {
  const outOfSync = diff.toAdd.length > 0 || diff.toRemove.length > 0
  if (outOfSync) {
    reporter.printPreSyncPreview(diff)
    reporter.printError(
      'Skills are out of sync',
      'Run `npx skills-sync` to fix'
    )
    process.exit(1)
  }
  // In sync
  console.log(pc.green('✓ Skills in sync'))
  process.exit(0)
}
```

### Mutual Exclusion Pattern

Follows existing `--global`/`--local` pattern:

```typescript
if (options.check && options.dryRun) {
  reporter.printError(
    '--check and --dry-run are mutually exclusive',
    '--check already performs a read-only comparison'
  )
  process.exit(1)
}
```

### npm Distribution Pattern

The `files` field in `package.json` is the standard approach (preferred over `.npmignore`):

```json
{
  "files": ["dist"],
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/<owner>/skills-sync.git"
  }
}
```

### Anti-Patterns to Avoid
- **Adding --check logic in executor.ts:** Check mode never executes — it belongs in the CLI entry point
- **Creating a separate check module:** Too small to justify — it's ~15 lines in the CLI action
- **Using .npmignore:** The `files` whitelist is more explicit and maintainable

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Diff preview output | Custom check formatter | `reporter.printPreSyncPreview()` | Already renders "To install:" and "To remove:" lists |
| Exit code management | Custom exit handler | `process.exit(0/1)` | Standard Node.js pattern, already used throughout |
| CLI flag definition | Manual argv parsing | `commander .option()` | Already used for all other flags |

## Common Pitfalls

### Pitfall 1: Forgetting Orphan Detection in Check Mode
**What goes wrong:** `--check` only checks `toAdd` but ignores `toRemove` (orphans)
**Why it happens:** Phase 1 mindset — orphans were deferred
**How to avoid:** Check `toAdd.length > 0 || toRemove.length > 0` — both indicate out-of-sync
**Warning signs:** Success criteria #1 explicitly mentions "including orphaned skills"

### Pitfall 2: Running Check After Prompts/Execution
**What goes wrong:** `--check` triggers confirmation prompts or subprocess execution
**Why it happens:** Check logic placed too late in the pipeline
**How to avoid:** Insert check logic immediately after `computeDiff()`, before orphan prompt

### Pitfall 3: Missing `files` Field in package.json
**What goes wrong:** `npm publish` includes test files, source TypeScript, planning docs
**Why it happens:** npm includes everything by default
**How to avoid:** Explicit `"files": ["dist"]` whitelist — only ship the built bundle

### Pitfall 4: Check + Verbose Interaction
**What goes wrong:** `--check --verbose` is accepted but meaningless (no commands to show)
**Why it happens:** No explicit exclusion check
**How to avoid:** Per CONTEXT.md: `--check` does NOT support `--verbose`. Can silently ignore since it has no effect, or error.

## Code Examples

### Adding --check to Commander
```typescript
// In bin/skills-sync.ts, on the program definition:
.option('--check', 'exit with code 1 if skills are out of sync (CI gate)')
```

### Check Mode Logic
```typescript
// After computeDiff(), before orphan prompt:
if (options.check) {
  const outOfSync = diff.toAdd.length > 0 || diff.toRemove.length > 0
  if (outOfSync) {
    reporter.printPreSyncPreview(diff)
    reporter.printError('Skills are out of sync', 'Run `npx skills-sync` to fix')
    process.exit(1)
  }
  console.log(pc.green('✓ Skills in sync'))
  process.exit(0)
}
```

### package.json Distribution Fields
```json
{
  "files": ["dist"],
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/<owner>/skills-sync.git"
  },
  "keywords": ["skills", "claude", "agents", "cli", "sync", "toml"]
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| .npmignore blacklist | `files` whitelist in package.json | npm 5+ (2017) | Safer, explicit, less likely to leak files |
| `prepublish` script | `prepublishOnly` + `prepare` | npm 5+ (2017) | Clearer lifecycle hooks |

## Open Questions

None — phase scope is well-defined and entirely within established patterns.

## Sources

### Primary (HIGH confidence)
- Codebase inspection: `bin/skills-sync.ts`, `src/diff.ts`, `src/reporter.ts`, `src/types.ts`, `package.json`, `tsup.config.ts`
- npm docs: `files` field, `bin` field, `repository` field — standard npm package.json fields

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies needed
- Architecture: HIGH - follows existing patterns exactly
- Pitfalls: HIGH - well-known patterns, small surface area

**Research date:** 2026-03-01
**Valid until:** 2026-04-01 (stable domain, no moving parts)
