# Phase 2: Orphan Detection + Project Scope - Research

**Researched:** 2026-03-01
**Domain:** TypeScript CLI — orphan detection, interactive prompts, source attribution file, subprocess removal, CLI flag extensions
**Confidence:** HIGH

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| STATE-03 | Tool reads `.skills-sync-sources.json` for project-scope source attribution (orphan detection) | Custom JSON file, skills-sync-owned schema; no upstream equivalent; design documented in Architecture Patterns |
| SYNC-03 | Tool detects orphaned skills and prompts user for removal (or auto-removes with `--yes`) | `computeDiff` already returns `toRemove: []` (Phase 1 stub); orphan = skill name in installed state not reachable from any manifest source; `--yes` check before any prompt |
| SYNC-04 | Tool removes orphaned skills by shelling out to `npx skills remove <name> --yes` | Verified via `npx skills remove --help`: accepts skill names as positional args + `--yes` / `-y` / `--global` flags |
| SYNC-05 | Tool writes `.skills-sync-sources.json` after successful project-scope installs for source attribution | Atomic write via tmp-file rename pattern; schema: `{ version: 1, sources: Record<source, string[]> }` |
| FLAG-03 | `--yes` skips confirmation prompts and passes `-y` to `npx skills` subcommands | Commander `.option('--yes')` or `-y`; must check BEFORE spawning any prompt; already passed through to `npx skills add` |
| FLAG-04 | `--verbose` shows the exact `npx skills` commands being executed | Phase 1 already calls `reporter.printCommand()` unconditionally; Phase 2 gates it behind `--verbose` flag |
| FLAG-05 | `--agent <AGENT>` passes `--agent` to `npx skills add` commands | `npx skills add <source> --agent <agent>` verified; also applies to `npx skills remove` |
| OUTP-02 | Tool shows pre-sync diff preview with confirmation prompt (unless `--yes`) | Orphan list printed; `@inquirer/prompts` `confirm()` for interactive prompt; guarded by `--yes` check |
</phase_requirements>

---

## Summary

Phase 2 extends the working Phase 1 pipeline with four distinct concerns: (1) source attribution tracking so orphans can be detected on future local-scope runs, (2) orphan detection itself using the existing `Diff.toRemove` stub, (3) removal via `npx skills remove <name> --yes`, and (4) three new CLI flags (`--yes`, `--verbose`, `--agent`). The phase also adds an interactive confirmation prompt for orphan removal in the default (non-CI) case.

The core architectural insight is that all four concerns hook into existing extension points that Phase 1 deliberately left open. The `Diff.toRemove` array is already declared in `types.ts` but always returns empty. The `reporter.printCommand()` method is already called inside `executor.ts` unconditionally — `--verbose` changes it from always-on to gated. The executor already accepts `SyncOptions` and threads them to `npx skills add`; `--agent` is simply one more arg appended there. The only genuinely new infrastructure is `@inquirer/prompts` for the confirmation UX and `.skills-sync-sources.json` as a skills-sync-owned attribution file.

The trickiest implementation concern is the CI/non-TTY interaction between `--yes` and `@inquirer/prompts`. The library throws `ExitPromptError` when stdin is not a TTY — meaning the `--yes` guard must fire BEFORE the prompt is invoked. This is not optional defensive coding; it is the primary correctness requirement for FLAG-03. Equally important: `npx skills remove <name>` takes skill directory names (e.g., `git-helpers`), not source URLs (e.g., `owner/repo`). The `.skills-sync-sources.json` file must therefore store the mapping from source to skill names so the executor knows what to pass to the remove command.

**Primary recommendation:** Build in this order: (1) extend `SyncOptions` with `yes`, `verbose`, `agent`; (2) write `.skills-sync-sources.json` after install; (3) read `.skills-sync-sources.json` in `readLocalState`; (4) populate `computeDiff` `toRemove` from orphaned skill names; (5) add removal to executor; (6) add prompt to CLI entry point; (7) gate `printCommand` behind `--verbose`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `@inquirer/prompts` | ^8.3.0 (latest 2026-03) | Interactive confirm prompt for orphan removal | ESM, actively maintained, single-package confirm import, AbortSignal support, standard Inquirer.js successor |
| `node:fs/promises` (built-in) | Node 20+ | Atomic write of `.skills-sync-sources.json` via tmp + rename | Already used throughout the codebase; no new dep needed |

All other runtime dependencies (commander, execa, picocolors, smol-toml) carry over from Phase 1 unchanged.

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@inquirer/prompts` | ^8.3.0 | `confirm()` prompt only | Only for the orphan-removal confirmation dialog; no other prompts in Phase 2 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `@inquirer/prompts` | `enquirer` | enquirer is unmaintained (last release 2021); Inquirer.js is the maintained successor |
| `@inquirer/prompts` | `@clack/prompts` | @clack/prompts 1.0.1 (released 15 days ago as of research date); too new, insufficient track record for this use case |
| `@inquirer/prompts` | `process.stdout.write` + `readline` | Works but requires significant hand-rolling of prompt UI (y/N default, cursor handling, Ctrl+C); not worth the LOC |
| `@inquirer/prompts` | Manual `stdin.read()` | Same hand-roll concern; Inquirer handles TTY detection and raw mode |

**Installation:**
```bash
npm install @inquirer/prompts
```

---

## Architecture Patterns

### Recommended Project Structure Extension

```
src/
├── types.ts           # Add yes, verbose, agent to SyncOptions; SourcesFile interface
├── state.ts           # readLocalState: read .skills-sync-sources.json → bySource
├── diff.ts            # computeDiff: populate toRemove from orphaned skill names
├── executor.ts        # executeSync: add removeSource(); gate printCommand on verbose
├── reporter.ts        # Add printOrphans(), printRemoving(), printRemoved()
├── sources.ts         # NEW: read/write .skills-sync-sources.json (source attribution)
└── bin/skills-sync.ts # Add --yes, --verbose, --agent flags; orphan confirmation prompt
test/
├── sources.test.ts    # NEW: unit tests for sources.ts read/write
├── state.test.ts      # ADD: tests for readLocalState with .skills-sync-sources.json
├── diff.test.ts       # ADD: tests for toRemove population
├── executor.test.ts   # ADD: tests for removeSource(), verbose gating, agent passthrough
└── reporter.test.ts   # ADD: tests for new reporter methods
```

### Pattern 1: SyncOptions Extension (Additive Only)

**What:** `SyncOptions` in `types.ts` gains three new optional fields. All existing call sites continue to work without changes because the fields are optional.

**When to use:** For all new Phase 2 flag threading.

**Example:**
```typescript
// src/types.ts
export interface SyncOptions {
  scope: 'global' | 'local'
  dryRun: boolean
  configPath?: string
  // Phase 2 additions:
  yes?: boolean       // --yes: skip prompts, pass -y to subcommands
  verbose?: boolean   // --verbose: show npx commands being run
  agent?: string      // --agent: passed to npx skills add/remove
}
```

### Pattern 2: .skills-sync-sources.json Schema and Atomic Write

**What:** skills-sync's own source attribution file for local scope. Maps normalized source strings to arrays of installed skill names. Written after every successful local-scope install. Read by `readLocalState` on subsequent runs to power orphan detection.

**Why this schema:** The remove command takes skill names (e.g., `git-helpers`), not source URLs. The diff engine needs to know which skill names belong to each source so it can compute orphaned skill names. The lock file confirms this: `source: "vercel-labs/agent-skills"` maps to N skills like `vercel-react-best-practices`, `vercel-composition-patterns`, etc.

**When to use:** Write after every successful install in local scope; read at start of local-scope sync.

**Example:**
```typescript
// src/sources.ts

/** Schema for .skills-sync-sources.json — skills-sync-owned attribution file */
export interface SourcesFile {
  version: 1
  /** normalized source → skill names installed from that source */
  sources: Record<string, string[]>
}

const SOURCES_FILE = '.skills-sync-sources.json'
const SOURCES_VERSION = 1

export async function readSourcesFile(
  basePath: string = process.cwd()
): Promise<SourcesFile | null> {
  const filePath = join(basePath, SOURCES_FILE)
  try {
    const raw = await readFile(filePath, 'utf-8')
    const parsed = JSON.parse(raw) as SourcesFile
    if (parsed.version !== SOURCES_VERSION) return null  // version mismatch → ignore
    return parsed
  } catch {
    return null  // missing or corrupt → orphan detection not available
  }
}

export async function writeSourcesFile(
  bySource: Map<string, string[]>,
  basePath: string = process.cwd()
): Promise<void> {
  const sources: Record<string, string[]> = {}
  for (const [source, names] of bySource) {
    sources[source] = names
  }
  const data: SourcesFile = { version: SOURCES_VERSION, sources }
  const filePath = join(basePath, SOURCES_FILE)
  const tmpPath = `${filePath}.tmp`
  // Atomic write: write to .tmp, then rename — avoids corrupt state on crash
  await writeFile(tmpPath, JSON.stringify(data, null, 2), 'utf-8')
  await rename(tmpPath, filePath)
}
```

### Pattern 3: readLocalState Extended with Source Attribution

**What:** `readLocalState` checks for `.skills-sync-sources.json` first. If found, it returns proper source → skill mappings (enabling orphan detection). If missing, it falls back to the Phase 1 `__local__` bucket (no orphan detection possible, install-all behavior preserved).

**When to use:** Always. The graceful fallback preserves backward compatibility.

**Example:**
```typescript
// src/state.ts — readLocalState updated
export async function readLocalState(
  basePath: string = process.cwd()
): Promise<InstalledState> {
  // Try source attribution file first
  const sourcesFile = await readSourcesFile(basePath)
  if (sourcesFile) {
    const bySource = new Map<string, string[]>()
    for (const [source, names] of Object.entries(sourcesFile.sources)) {
      bySource.set(source, names)
    }
    return { bySource }
  }

  // Fallback: scan filesystem (Phase 1 behavior)
  // Returns bySource with '__local__' key — diff produces toAdd = all manifest sources
  return readLocalStateFromFilesystem(basePath)
}
```

### Pattern 4: computeDiff Extended with toRemove

**What:** `computeDiff` now populates `toRemove` with skill names (not source URLs) that are installed but not reachable from any manifest source. These are the names passed to `npx skills remove <name>`.

**Critical nuance:** `toRemove` contains skill *names* (e.g., `git-helpers`), NOT source URLs. This is because `npx skills remove` accepts skill names as positional arguments, not source URLs. The mapping from source to skill names comes from `InstalledState.bySource`.

**When to use:** Always, but only when `InstalledState.bySource` has real source keys (not `__local__`). When the installed state uses `__local__`, skip orphan detection entirely.

**Example:**
```typescript
// src/diff.ts — computeDiff updated
export function computeDiff(manifest: Manifest, state: InstalledState): Diff {
  const installedKeys = new Set<string>()
  for (const key of state.bySource.keys()) {
    installedKeys.add(normalizeSource(key))
  }

  const manifestKeys = new Set(manifest.sources.map(normalizeSource))

  const toAdd: string[] = []
  const unchanged: string[] = []

  for (const source of manifest.sources) {
    if (installedKeys.has(normalizeSource(source))) {
      unchanged.push(source)
    } else {
      toAdd.push(source)
    }
  }

  // Orphan detection: skill names from sources not in manifest
  // Skip if state uses __local__ (no source attribution available)
  const toRemove: string[] = []
  if (!state.bySource.has('__local__')) {
    for (const [source, skillNames] of state.bySource) {
      if (!manifestKeys.has(normalizeSource(source))) {
        toRemove.push(...skillNames)
      }
    }
  }

  return { toAdd, unchanged, toRemove }
}
```

### Pattern 5: Orphan Removal in Executor

**What:** `executeSync` now handles `diff.toRemove`. Before executing removes, the CLI entry point checks `--yes` or prompts the user. Removal uses `npx skills remove <name> --yes [--global] [--agent <agent>]`.

**Key finding from `npx skills remove --help`:** The command takes skill names as positional arguments (space-separated or repeated calls). The `--yes` / `-y` flag suppresses the interactive selection menu in `npx skills remove`. The `--agent` flag targets a specific agent.

**When to use:** When `diff.toRemove.length > 0` and user has confirmed (or `--yes` is set).

**Example:**
```typescript
// src/executor.ts — removeSource added
export async function removeSkill(
  skillName: string,
  options: SyncOptions
): Promise<boolean> {
  const args = ['skills', 'remove', skillName, '--yes']
  if (options.scope === 'global') args.push('--global')
  if (options.agent) args.push('--agent', options.agent)

  const cmd = `npx ${args.join(' ')}`
  if (options.verbose) reporter.printCommand(cmd)

  if (options.dryRun) {
    reporter.printDryRunCommand(cmd)
    return true
  }

  reporter.printRemoving(skillName)
  try {
    await execa('npx', args, { stdout: 'inherit', stderr: 'inherit' })
    reporter.printRemoved(skillName)
    return true
  } catch (err) {
    const reason = err instanceof ExecaError ? err.message : String(err)
    reporter.warnRemoveFailed(skillName, reason)
    return false
  }
}
```

### Pattern 6: --verbose Gate in Executor

**What:** Phase 1 calls `reporter.printCommand()` unconditionally before every `execa` call. Phase 2 gates it behind `options.verbose`. This is a one-line change in `executor.ts`.

**Example:**
```typescript
// src/executor.ts — verbose gate (replace unconditional printCommand)
if (options.verbose) reporter.printCommand(cmd)
```

### Pattern 7: --yes Guard Before Prompt (CI Safety)

**What:** In `bin/skills-sync.ts`, the orphan confirmation must check `options.yes` BEFORE calling `@inquirer/prompts`. If `--yes` is set OR stdin is not a TTY, skip the prompt. `@inquirer/prompts` throws `ExitPromptError` when stdin is not a TTY — catching this as a fallback is acceptable but not the primary defense.

**When to use:** Always. This is the correctness requirement for FLAG-03.

**Example:**
```typescript
// bin/skills-sync.ts — orphan confirmation flow
import { confirm } from '@inquirer/prompts'

// After computing diff, before executing:
if (diff.toRemove.length > 0) {
  reporter.printOrphans(diff.toRemove)

  let shouldRemove = options.yes ?? false

  if (!shouldRemove) {
    if (!process.stdin.isTTY) {
      // Non-interactive: skip removal, warn user to re-run with --yes
      reporter.warnSkippingOrphans('stdin is not a TTY; use --yes to remove orphans in CI')
      diff = { ...diff, toRemove: [] }
    } else {
      try {
        shouldRemove = await confirm({
          message: `Remove ${diff.toRemove.length} orphaned skill(s)?`,
          default: false,
        })
      } catch {
        // ExitPromptError or other abort — treat as "no"
        shouldRemove = false
      }
    }

    if (!shouldRemove) {
      diff = { ...diff, toRemove: [] }
    }
  }
}
```

### Pattern 8: Writing .skills-sync-sources.json After Install

**What:** After `executeSync` completes local-scope installs, skills-sync must discover which skill names were installed and record them in `.skills-sync-sources.json`. The challenge: `npx skills add` does not return which skill names it installed. The tool must scan the filesystem AFTER install to detect new skill directories, then attribute them to the source that was just installed.

**Approach:** Snapshot skill directories before install; snapshot after; diff to find new skill names; attribute to the installed source.

**Example:**
```typescript
// src/sources.ts — helper to detect newly installed skills
export async function detectInstalledSkills(
  basePath: string = process.cwd()
): Promise<string[]> {
  const scanDirs = ['.agents/skills', '.claude/skills']
  const names = new Set<string>()
  for (const relDir of scanDirs) {
    try {
      const entries = await readdir(join(basePath, relDir), { withFileTypes: true })
      entries.filter(d => d.isDirectory()).forEach(d => names.add(d.name))
    } catch { /* skip */ }
  }
  return [...names]
}
```

After all installs for a source: `newSkills = after.filter(n => !before.includes(n))`. Associate `source → newSkills` in the running map, then write `.skills-sync-sources.json`.

### Anti-Patterns to Avoid

- **Calling `@inquirer/prompts` without a TTY guard:** Always check `process.stdin.isTTY` and `options.yes` before invoking `confirm()`. The library throws `ExitPromptError` in CI — catching this after the fact works but is confusing and emits visible errors.
- **Passing source URLs to `npx skills remove`:** The remove command accepts skill directory names (e.g., `git-helpers`), not source URLs. Passing `vercel-labs/agent-skills` will fail silently or with an unhelpful error.
- **Writing `.skills-sync-sources.json` before install succeeds:** Write only after the install call succeeds. On partial failure (some sources failed), write the sources that succeeded.
- **Blocking the `toRemove` path when `.skills-sync-sources.json` is absent:** If the file is absent, `toRemove` must stay empty. Never attempt orphan detection with the `__local__` bucket — skill names in that bucket cannot be attributed to sources.
- **Gating verbose on `dryRun`:** `--verbose` and `--dry-run` are independent flags. Verbose shows commands even when actually running them; it is not just a dry-run detail.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Interactive confirm prompt | `readline`/raw stdin | `@inquirer/prompts confirm()` | TTY detection, raw mode, Ctrl+C handling (ExitPromptError), cursor handling all handled |
| Atomic file write | Direct `writeFile` | Write to `.tmp` then `rename()` | `writeFile` is not atomic — crash mid-write corrupts `.skills-sync-sources.json`, breaking orphan detection on next run |
| Skill name detection post-install | Parse `npx skills` stdout | Filesystem scan diff (before/after) | `npx skills add` does not document machine-readable output; stdout format can change |

**Key insight:** The `.skills-sync-sources.json` problem looks novel but decomposes into two solved sub-problems: atomic JSON file write (rename pattern) and before/after directory scan diff.

---

## Common Pitfalls

### Pitfall 1: `@inquirer/prompts` Throws in CI (stdin not a TTY)
**What goes wrong:** `confirm()` is called in CI where stdin is piped, not a TTY. The library throws `ExitPromptError`. The `catch` block may not handle it correctly, or the error leaks to the top-level handler which reports it as an unexpected failure.
**Why it happens:** `@inquirer/prompts` requires an interactive terminal for keypress handling. When spawned as a subprocess or with piped stdin, raw mode setup fails.
**How to avoid:** Check `process.stdin.isTTY` BEFORE calling `confirm()`. If not a TTY and `--yes` is not set, emit a warning and skip removal. Never rely solely on try/catch of `ExitPromptError`.
**Warning signs:** `skills-sync` exits with non-zero in CI with an `ExitPromptError` stack trace; CI jobs that should auto-remove orphans with `--yes` instead hang.

### Pitfall 2: Passing Source URLs to `npx skills remove`
**What goes wrong:** `diff.toRemove` contains source URLs (e.g., `owner/repo`) but `npx skills remove` expects skill directory names (e.g., `git-helpers`). The command silently does nothing or errors.
**Why it happens:** Confusion between the two levels of abstraction: manifest tracks sources, but the remove command operates on skill names.
**How to avoid:** `toRemove` in `Diff` MUST contain skill directory names, not source URLs. Populate it from `state.bySource[orphanedSource]` (the value array), not the keys.
**Warning signs:** `npx skills remove owner/repo` exits 0 but the skill directory still exists on disk.

### Pitfall 3: `.skills-sync-sources.json` Written on Partial Failure
**What goes wrong:** 3 of 5 sources succeed, 2 fail. The file is written with all 5 sources as "installed". On the next run, the 2 failed sources appear as "unchanged" and are never retried.
**Why it happens:** Writing the file before checking individual source results.
**How to avoid:** Build the `bySource` map incrementally. Only add a source to the map after its `npx skills add` call succeeds. Write the file once at the end with only successful sources.
**Warning signs:** Sources that failed to install are never retried on subsequent runs; user must manually delete `.skills-sync-sources.json` to force reinstall.

### Pitfall 4: `--verbose` Always Printing Commands (Breaking Change to Phase 1 Behavior)
**What goes wrong:** Phase 1 calls `reporter.printCommand()` unconditionally. Phase 2 requires this be gated behind `--verbose`. If the gate is added inconsistently (some call sites gated, some not), verbose output is partial.
**Why it happens:** There is only one call site in `executor.ts` — this is a single-line change. But if Phase 2 adds new subcommand invocations (e.g., `removeSkill`) and forgets the verbose gate on those, the flag is inconsistent.
**How to avoid:** `reporter.printCommand()` is called ONLY inside `executor.ts`. All executor functions receive `SyncOptions` and apply the same `if (options.verbose)` guard. Never call `reporter.printCommand()` from the CLI entry point or other modules.
**Warning signs:** Commands printed even when `--verbose` is not set; or commands not printed when `--verbose` IS set.

### Pitfall 5: Orphan Detection Running Without Source Attribution
**What goes wrong:** `.skills-sync-sources.json` is absent (first-ever local sync). `computeDiff` tries to detect orphans from the `__local__` filesystem scan. All locally installed skills appear as orphans of unknown sources. The tool prompts to remove all of them.
**Why it happens:** Not checking for the `__local__` sentinel key before populating `toRemove`.
**How to avoid:** In `computeDiff`, check `state.bySource.has('__local__')`. If true, set `toRemove = []` unconditionally. Only run orphan detection when `bySource` has real source keys from `.skills-sync-sources.json`.
**Warning signs:** On first-ever `--local` sync, the tool offers to remove all installed skills; users confused about why installed skills are listed as orphans.

### Pitfall 6: `--agent` Flag Passed to `npx skills remove` Without Verification
**What goes wrong:** `--agent` is forwarded to `npx skills remove` calls, but the skill was installed to all agents (no `--agent` originally). Passing `--agent claude-code` to the remove command only removes from one agent, leaving the skill in others.
**Why it happens:** The `--agent` flag controls agent targeting consistently across add and remove, but the scope is asymmetric when skills were originally installed without `--agent`.
**How to avoid:** Accept this limitation for Phase 2 — document it. If `--agent` is provided, pass it to both add and remove. The behavior is consistent and predictable; the user who passes `--agent` understands the targeting scope.
**Warning signs:** Skill still visible in `.claude/skills/` after remove with `--agent cursor`.

---

## Code Examples

### `npx skills remove` — Verified Command Surface

From `npx skills remove --help` (verified on local machine):
```
Usage: skills remove [skills...] [options]

Arguments:
  skills      Optional skill names to remove (space-separated)

Options:
  -g, --global   Remove from global scope (~//) instead of project scope
  -a, --agent    Remove from specific agents (use '*' for all agents)
  -s, --skill    Specify skills to remove (use '*' for all skills)
  -y, --yes      Skip confirmation prompts
  --all          Shorthand for --skill '*' --agent '*' -y

Examples:
  $ skills remove my-skill                   # remove specific skill
  $ skills remove skill1 skill2 -y           # remove multiple skills without prompt
  $ skills remove --global my-skill          # remove from global scope
  $ skills rm --agent claude-code my-skill   # remove from specific agent
```

**Key finding:** Skill names are positional arguments (not `--name` flags). Multiple skills in one call: `npx skills remove skill-a skill-b --yes`. This means for N orphans, you can call once with all names, rather than N individual calls.

### Lock File Schema (Verified on Local Machine)

From inspecting `~/.agents/.skill-lock.json` (version 3):
```json
{
  "version": 3,
  "skills": {
    "vercel-react-best-practices": {
      "source": "vercel-labs/agent-skills",
      "sourceType": "github",
      "sourceUrl": "https://github.com/vercel-labs/agent-skills.git",
      "skillPath": "skills/react-best-practices/SKILL.md",
      "skillFolderHash": "079b06a135401704ea94ab74c7c9d86c92ba259f",
      "installedAt": "2026-02-26T10:00:59.047Z",
      "updatedAt": "2026-02-26T10:00:59.047Z"
    },
    "find-skills": {
      "source": "vercel-labs/skills",
      ...
    }
  }
}
```

This confirms: one source can produce N skills. `state.bySource.get('vercel-labs/agent-skills')` = `['vercel-react-best-practices', 'vercel-composition-patterns', ...]`.

### `@inquirer/prompts` Confirm Usage

```typescript
// Source: github.com/SBoudrias/Inquirer.js — confirmed ESM import
import { confirm } from '@inquirer/prompts'

// Always guard with TTY check FIRST
if (!process.stdin.isTTY) {
  // Cannot show prompt — handle programmatically
  return
}

let shouldRemove: boolean
try {
  shouldRemove = await confirm({
    message: `Remove ${orphanCount} orphaned skill(s)? (y/N)`,
    default: false,
  })
} catch {
  // ExitPromptError (Ctrl+C) — treat as "no"
  shouldRemove = false
}
```

### `.skills-sync-sources.json` Schema

```json
{
  "version": 1,
  "sources": {
    "owner/repo": ["skill-name-a", "skill-name-b"],
    "other-owner/other-repo": ["skill-name-c"]
  }
}
```

- Keys are normalized source strings (lowercase, trimmed, no trailing slash) — matches `normalizeSource()` from `state.ts`
- Values are skill directory names as they appear in `.agents/skills/` or `.claude/skills/`
- `version: 1` allows future format evolution

### Commander Flag Additions

```typescript
// bin/skills-sync.ts — new flags for Phase 2
program
  // existing flags preserved...
  .option('-y, --yes', 'skip confirmation prompts; pass -y to npx skills subcommands')
  .option('--verbose', 'show the exact npx skills commands being executed')
  .option('--agent <agent>', 'target a specific agent for all installs and removals')
```

Note: `--yes` should accept both `-y` and `--yes` (commander shorthand syntax `-y, --yes`).

### OUTP-02: Pre-Sync Diff Preview

```typescript
// reporter.ts additions
printOrphans(skillNames: string[]): void {
  console.log(`\n${pc.yellow('Orphaned skills (installed but not in manifest):')}`)
  for (const name of skillNames) {
    console.log(`  ${pc.red('✗')} ${name}`)
  }
}

printRemoving(skillName: string): void {
  console.log(`  ${pc.red('↓')} removing ${skillName}`)
}

printRemoved(skillName: string): void {
  console.log(`  ${pc.dim('✓')} removed ${skillName}`)
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| enquirer (interactive prompts) | `@inquirer/prompts` | enquirer abandoned ~2021 | @inquirer/prompts is the maintained Inquirer.js successor; ESM-first |
| `fs.writeFileSync` for config files | `writeFile` + `rename` (atomic) | Node.js 12+ | Atomic write prevents corrupt state on crash |
| Polling filesystem for state | Read own attribution file | Design choice | Attribution file enables source-level diff without querying upstream |

**Deprecated/outdated:**
- enquirer: Last published 2021, treat as unmaintained — do not use
- `writeFile` without atomic rename for critical state files: can produce corrupt JSON if process crashes mid-write

---

## Open Questions

1. **Skill name discovery after `npx skills add` (MEDIUM confidence)**
   - What we know: `npx skills add` does not document machine-readable output. The filesystem before/after scan diff approach is the safest fallback.
   - What's unclear: Whether `npx skills add` outputs installed skill names to stdout in a parseable format that could be used instead of filesystem scanning.
   - Recommendation: Use filesystem scan diff (before/after). It is more robust and doesn't depend on `npx skills` output format stability. Scan both `.agents/skills/` and `.claude/skills/` for new directories after each source install.

2. **Multiple orphans: one `npx skills remove` call vs N calls (HIGH confidence)**
   - What we know: `npx skills remove` accepts multiple skill names as positional args (`skills remove skill-a skill-b -y`). Verified from `--help`.
   - What's unclear: Nothing — batch remove is supported.
   - Recommendation: Call `npx skills remove skill-a skill-b skill-c --yes` in a single call for efficiency. This avoids N subprocess spawns for N orphans.

3. **`.skills-sync-sources.json` should be gitignored or committed? (MEDIUM confidence)**
   - What we know: The file is local state that enables orphan detection. If committed, it serves as a shared source of truth for teams. If gitignored, each developer must sync once before orphan detection works.
   - What's unclear: The intended multi-developer workflow — is orphan detection per-developer or shared?
   - Recommendation: Do NOT gitignore it by default. Write it to the project root alongside `.skills.toml`. Teams that commit `.skills.toml` will naturally commit `.skills-sync-sources.json` alongside it. Add a note in `--init` template output. This defers the gitignore question to the user.

4. **`--agent` flag interaction with `npx skills remove` (MEDIUM confidence)**
   - What we know: `npx skills remove --agent <agent>` removes from a specific agent only.
   - What's unclear: If a skill was installed to multiple agents, `--agent claude-code remove` leaves it in other agents. This is consistent behavior but may surprise users.
   - Recommendation: Accept this in Phase 2. Document the behavior. Phase 3 or a future phase could add `--agent '*'` support.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | vitest v4.0.18 |
| Config file | `vitest.config.ts` — `include: ['test/**/*.test.ts']` |
| Quick run command | `npm test` (vitest run) |
| Full suite command | `npm test` (same — 226ms total) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STATE-03 | `readLocalState` reads `.skills-sync-sources.json` when present | unit | `npm test -- --run test/state.test.ts` | ✅ (add cases) |
| STATE-03 | `readLocalState` falls back to filesystem scan when file absent | unit | `npm test -- --run test/state.test.ts` | ✅ (add cases) |
| STATE-03 | `readSourcesFile` returns `null` on version mismatch | unit | `npm test -- --run test/sources.test.ts` | ❌ Wave 0 |
| SYNC-03 | `computeDiff` populates `toRemove` with orphaned skill names | unit | `npm test -- --run test/diff.test.ts` | ✅ (add cases) |
| SYNC-03 | `computeDiff` skips orphan detection when state has `__local__` | unit | `npm test -- --run test/diff.test.ts` | ✅ (add cases) |
| SYNC-04 | `removeSkill()` calls `npx skills remove <name> --yes` | unit | `npm test -- --run test/executor.test.ts` | ✅ (add cases) |
| SYNC-04 | `removeSkill()` passes `--global` in global scope | unit | `npm test -- --run test/executor.test.ts` | ✅ (add cases) |
| SYNC-04 | `removeSkill()` passes `--agent` when provided | unit | `npm test -- --run test/executor.test.ts` | ✅ (add cases) |
| SYNC-05 | `writeSourcesFile` writes correct JSON with atomic rename | unit | `npm test -- --run test/sources.test.ts` | ❌ Wave 0 |
| SYNC-05 | `writeSourcesFile` only writes sources that succeeded | unit | `npm test -- --run test/sources.test.ts` | ❌ Wave 0 |
| FLAG-03 | `--yes` skips prompt; passes `-y` to `npx skills add` | unit | `npm test -- --run test/executor.test.ts` | ✅ (add case) |
| FLAG-04 | `--verbose` gates `reporter.printCommand()` | unit | `npm test -- --run test/executor.test.ts` | ✅ (add case) |
| FLAG-05 | `--agent` passed to `npx skills add` args | unit | `npm test -- --run test/executor.test.ts` | ✅ (add case) |
| OUTP-02 | `reporter.printOrphans()` emits orphan list | unit | `npm test -- --run test/reporter.test.ts` | ✅ (add cases) |
| OUTP-02 | Confirm prompt skipped when `--yes` or stdin not TTY | manual-only | manual test in non-TTY shell | N/A — requires real TTY |

### Sampling Rate

- **Per task commit:** `npm test`
- **Per wave merge:** `npm test` (full suite — 226ms)
- **Phase gate:** Full suite green + `npm run typecheck` clean before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `test/sources.test.ts` — covers STATE-03 (readSourcesFile), SYNC-05 (writeSourcesFile); new file needed
- [ ] `test/fixtures/skills-sync-sources.json` — sample attribution file fixture for state.test.ts and sources.test.ts

*(No framework install needed — vitest is already configured and running)*

---

## Sources

### Primary (HIGH confidence)

- `npx skills --help` and `npx skills remove --help` — verified locally; exact flag surface for add and remove commands
- `~/.agents/.skill-lock.json` — inspected directly on local machine; confirms version 3 schema with `source`, `skillPath`, `skillFolderHash` fields; confirms one source → multiple skills mapping
- Phase 1 source files (`src/types.ts`, `src/executor.ts`, `src/state.ts`, `src/diff.ts`, `src/reporter.ts`, `bin/skills-sync.ts`) — reviewed in full; Phase 2 extension points confirmed
- `.planning/phases/01-core-sync-pipeline/01-04-SUMMARY.md` — documents Phase 1 patterns established and Phase 2 extension points
- [github.com/SBoudrias/Inquirer.js](https://github.com/SBoudrias/Inquirer.js) — confirm prompt ESM import, ExitPromptError behavior confirmed
- [DeepWiki: vercel-labs/skills](https://deepwiki.com/vercel-labs/skills/2-getting-started) — confirms project scope has no upstream lock file; `addSkillToLock()` only runs for `installGlobally === true`

### Secondary (MEDIUM confidence)

- WebSearch: `@inquirer/prompts` v8.3.0 — latest version confirmed; ESM-first; ExitPromptError on Ctrl+C; TTY requirement
- WebSearch: Inquirer.js issue #495 — confirms `prompt()` fails when stdin is not TTY; `isTtyError` property on older versions; current version throws `ExitPromptError`

### Tertiary (LOW confidence)

- None — all critical findings verified with primary sources

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — `@inquirer/prompts` is the clear choice; verified version; npm data confirms; no viable alternatives
- Architecture: HIGH — Phase 1 extension points match Phase 2 requirements exactly; `npx skills remove` flags verified locally
- Pitfalls: HIGH — TTY pitfall verified via Inquirer.js issue tracker + local `npx skills remove --help`; `toRemove` naming pitfall is architectural (verified by inspecting lock file schema); partial failure write pitfall is a standard atomic write concern

**Research date:** 2026-03-01
**Valid until:** 2026-04-01 (stable stack; `npx skills` CLI flags confirmed locally; 30-day window appropriate)
