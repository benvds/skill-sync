# Phase 1: Core Sync Pipeline - Research

**Researched:** 2026-03-01
**Domain:** TypeScript CLI — declarative TOML manifest parsing, state reading, diff computation, subprocess delegation to `npx skills`
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Output format and reporting
- Default output is colored terminal text (not JSON)
- Summary line always printed: "X installed, Y unchanged" (or "X to install, Y unchanged" for dry-run)
- Use status symbols: ✓ (present/done), ✗ (error/skipped), ↓ (installing), ○ (pending)
- Errors include remediation hints: missing manifest → `--init` hint, invalid TOML → line/column, `npx skills` unavailable → install hint
- No spinner/progress bar for v1 — keep it simple, just print each action as it happens

#### Flag design
- `--global` and `--local` are mutually exclusive; default is `--local`
- `--dry-run` prints planned actions and exits without executing
- `--init` creates sample `.skills.toml` in CWD (or `~/.config/skills-sync/skills.toml` with `--global`)
- `--config <PATH>` overrides manifest discovery
- `-h`/`--help` and `-V`/`--version` standard flags via commander

#### Manifest discovery order
1. Explicit `--config <path>` flag
2. `.skills.toml` in CWD (for `--local`, which is default)
3. `~/.config/skills-sync/skills.toml` (for `--global`)
4. Error with hint to run `npx skills-sync --init`

#### Lock file handling
- Read `~/.agents/.skill-lock.json` for global scope — never write to it
- Implement `EXPECTED_LOCK_VERSION` check from day one; warn and fall back gracefully on mismatch
- For local scope, scan `.agents/skills/` and `.claude/skills/` directories for installed skill directories
- If lock file is missing/unreadable, treat as empty state (everything needs installing)

#### Subprocess execution
- Always pass `--yes` to `npx skills add` (even though Phase 1 doesn't expose `--yes` flag to user yet)
- Always pass `--global` flag when in global scope
- Use execa for subprocess — explicit stderr passthrough, no shell injection risk
- If `npx skills add` fails for one source, warn and continue with remaining sources
- Show the exact command being run (this becomes `--verbose` only in Phase 2, but implement the plumbing now)

#### Source validation
- Sources in manifest are passed through as-is to `npx skills add` — no strict `owner/repo` format validation
- Validate: non-empty string, no duplicates in manifest
- Let `npx skills` handle format validation (GitHub, GitLab, local paths, URLs all supported)

### Claude's Discretion
- Exact project scaffold structure (src/ layout, tsconfig options)
- Commander program setup and help text formatting
- Internal error type design
- Test fixture design and test structure

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MNFST-01 | User can parse `.skills.toml` with clear error messages including line/column on syntax errors | smol-toml 1.6.0 provides TOML 1.1.0 parsing with line/column errors natively; see Code Examples §TOML Parsing |
| MNFST-02 | User can run `--init` to scaffold a sample `.skills.toml` with documented comments and usage instructions | InitCommand module (src/init.ts) is independent of all other modules; template string from PRD-v2.md |
| MNFST-03 | User can specify custom manifest path via `--config <PATH>` | commander 14 `--config <PATH>` option; manifest.ts resolveManifestPath() checks --config first |
| MNFST-04 | Tool discovers manifest in order: explicit `--config`, `.skills.toml` in CWD (for `--local`), `~/.config/skills-sync/skills.toml` (for `--global`) | manifest.ts config discovery flow; three-step fallback with locked order from CONTEXT.md |
| STATE-01 | Tool reads Vercel's `~/.agents/.skill-lock.json` for global-scope installed skills with schema version validation | state.ts reads JSON file; EXPECTED_LOCK_VERSION=3 (from PRD v2 — "stable at version 3"); guard missing file → empty state |
| STATE-02 | Tool scans `.agents/skills/` and `.claude/skills/` filesystem for project-scope installed skills | state.ts fs.readdirSync scan; no lock file available for local scope (confirmed upstream gap, issue #337) |
| SYNC-01 | Tool computes diff: missing sources (in manifest, not installed), orphaned skills (installed, not in manifest), present (in both) | diff.ts pure function; Phase 1 only needs toAdd + unchanged; orphan detection is Phase 2 |
| SYNC-02 | Tool installs missing skills by shelling out to `npx skills add <source> --yes` with appropriate scope flags | executor.ts execa('npx', ['skills', 'add', source, '--yes', ...scope flags]); always pass --yes per locked decision |
| FLAG-01 | `--global` and `--local` flags are mutually exclusive; default is `--local` | commander 14 option groups or manual mutual exclusion check; default scope = 'local' |
| FLAG-02 | `--dry-run` shows planned changes without executing | executor.ts dry-run boundary: check flag, print command, return early; no other module needs dry-run awareness |
| OUTP-01 | Tool prints colored summary: X installed, Y removed, Z unchanged | reporter.ts with picocolors; format: "N installed, N unchanged" for Phase 1 (no removal in Phase 1) |
| OUTP-03 | Error messages include remediation hints (missing manifest → `--init`, invalid TOML → line/column, `npx skills` unavailable → install hint) | errors.ts typed error classes; each error class carries a `hint` string printed below the error message |
</phase_requirements>

---

## Summary

Phase 1 builds the complete end-to-end sync pipeline for the install case: parse `.skills.toml`, read actual installed state (global lock file or local filesystem scan), compute the diff of what's missing, shell out to `npx skills add` for each missing source, and print colored summary output. It also includes the `--init` scaffold command, `--dry-run` flag, `--global`/`--local` scope flags, `--config <PATH>` override, and error messages with remediation hints. Orphan detection and removal are explicitly deferred to Phase 2.

The architecture is a linear typed pipeline — five source modules (manifest.ts, state.ts, diff.ts, executor.ts, reporter.ts) each own one stage and pass typed data structures forward. The critical architectural decisions are: (1) the diff computation is a pure function with no I/O, making it trivially testable; (2) all subprocess I/O is isolated to executor.ts, making `--dry-run` a single boundary check rather than a flag threaded through every module; and (3) the global lock file is read-only and checked for schema version on every run. This is a greenfield TypeScript rewrite — no Rust code is reused, though the `.skills.toml` format and conceptual module structure carry over.

The top risks for Phase 1 are: (a) reading the undocumented lock file without a schema version guard, which must be built from day one; (b) subprocess errors from `npx skills add` being swallowed instead of surfaced, which requires explicit stderr passthrough via execa from the start; and (c) TTY hangs when `npx skills` expects interactive input, mitigated by always passing `--yes` (per locked decision). All three are mitigable in initial implementation if the executor layer is built defensively. Project-level research confirms these risks and their mitigations are well-understood.

**Primary recommendation:** Build in strict module order (types → errors → manifest → diff → state → reporter → executor → init → bin) so each step delivers a testable artifact before wiring to the next stage.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| TypeScript | 5.x (latest stable) | Language | Type safety catches flag/option mismatches at compile time; ecosystem alignment with `npx skills` (TypeScript-first) |
| Node.js | 20+ (LTS) | Runtime | commander v14 hard requires Node 20+; LTS; native `fetch` reduces dep count |
| commander | 14.0.3 | CLI argument/option parsing | 500M weekly downloads; zero deps; 180 KB; mutual exclusion support; full TypeScript types |
| smol-toml | 1.6.0 | TOML manifest parsing | TOML 1.1.0 compliant; ~7M weekly downloads; zero deps; line/column error messages; last release Dec 2025 |
| execa | 9.6.1 | Shell-out to `npx skills` | No shell injection risk (array API); promise-based; explicit stdout/stderr streams; TypeScript types |
| picocolors | 1.1.1 | Terminal color output | CJS + ESM (unlike chalk v5 which is ESM-only); zero deps; 14x smaller than chalk; respects NO_COLOR |
| tsup | 8.5.1 | Build / bundle for npm | Zero-config dual CJS+ESM output; handles `bin` shebang; esbuild-powered; standard for TypeScript CLI packages |
| vitest | 2.x (latest) | Test runner | Native TypeScript support; no babel/ts-jest config; Jest-compatible API; fast |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @types/node | latest | Node.js type definitions | Required for fs, process, path, os usage in TypeScript |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| commander 14 | oclif | oclif is 12 MB / 30+ deps — overkill for a ~500 LOC thin wrapper |
| commander 14 | yargs | yargs is 850 KB vs commander's 180 KB for the same flag surface |
| smol-toml | @iarna/toml | @iarna/toml is unmaintained since 2021; only TOML 0.5 compliant |
| picocolors | chalk v5 | chalk v5 is ESM-only; complicates CJS/ESM dual output via tsup |
| execa | child_process (native) | native requires manual promise wrapping, error handling, stdout capture |
| vitest | jest | jest requires babel or ts-jest for TypeScript; significantly slower |

**Installation:**
```bash
# Runtime dependencies
npm install commander smol-toml execa picocolors

# Dev dependencies
npm install -D typescript tsup vitest @types/node
```

---

## Architecture Patterns

### Recommended Project Structure
```
skills-sync/
├── bin/
│   └── skills-sync.ts        # Entry point: shebang, parse argv, route to command
├── src/
│   ├── types.ts              # Shared TypeScript types (Manifest, InstalledState, Diff, SyncOptions)
│   ├── errors.ts             # Typed error classes with user-facing messages and remediation hints
│   ├── manifest.ts           # Read + validate .skills.toml (config discovery + TOML parse)
│   ├── diff.ts               # Pure diff computation: (Manifest, InstalledState) => Diff
│   ├── state.ts              # Read lock file (global) or scan filesystem (local) → InstalledState
│   ├── reporter.ts           # Terminal output: status symbols, colors, summary counts
│   ├── executor.ts           # All subprocess invocation; dry-run boundary
│   └── init.ts               # --init: generate sample .skills.toml template
├── test/
│   ├── manifest.test.ts      # Unit tests for TOML parsing and config discovery
│   ├── state.test.ts         # Unit tests for lock file parsing and filesystem scan
│   ├── diff.test.ts          # Unit tests for pure diff function (most important)
│   └── fixtures/
│       ├── valid.skills.toml
│       ├── invalid.skills.toml
│       └── skill-lock.json   # Sample Vercel lock file at EXPECTED_LOCK_VERSION=3
├── package.json
├── tsconfig.json
└── .skills.toml              # Skills-sync eats its own dog food
```

### Build Order (Mandatory — Each Step Testable)
1. `src/types.ts` — all shared types; no dependencies
2. `src/errors.ts` — typed error classes with hint strings; no dependencies
3. `src/manifest.ts` — TOML parsing and config discovery; depends on types + errors
4. `src/diff.ts` — pure diff computation; depends on types only; build with tests
5. `src/state.ts` — lock file + filesystem read; depends on types + errors
6. `src/reporter.ts` — terminal output with picocolors; depends on types
7. `src/executor.ts` — subprocess with execa; dry-run boundary; depends on types + reporter + errors
8. `src/init.ts` — template string generation; no dependencies; build anytime
9. `bin/skills-sync.ts` — commander wiring; build last

### Pattern 1: Pipeline with Typed Stage Outputs

**What:** Each pipeline stage returns a typed data structure consumed by the next stage. No global state, no mutable context — everything passed explicitly.

**When to use:** Always for this tool. The linear flow (parse → read → diff → execute → report) maps to explicit function calls.

**Example:**
```typescript
// Source: PRD-v2.md data flow + ARCHITECTURE.md Pattern 1

// src/types.ts
export interface Manifest {
  sources: string[]
}

export interface InstalledState {
  // key: normalized source string, value: list of skill names from that source
  bySource: Map<string, string[]>
}

export interface Diff {
  toAdd: string[]       // sources to install (in manifest, not installed)
  toRemove: string[]    // skill names to remove (orphans) — Phase 2
  unchanged: string[]   // sources already present
}

export interface SyncOptions {
  scope: 'global' | 'local'
  dryRun: boolean
  configPath?: string
}

// bin/skills-sync.ts — main sync flow
const manifest = await readManifest(options.configPath, options.scope)
const state    = await readActualState(options.scope)
const diff     = computeDiff(manifest, state)
reporter.printDiffPreview(diff)
if (options.dryRun) process.exit(0)
await execute(diff, options)
reporter.printSummary(diff)
```

### Pattern 2: Pure Diff Function

**What:** `computeDiff(manifest, state)` is a pure function — no I/O, no side effects, no flags threading. Compares desired sources against actual installed sources.

**When to use:** Always. This is the most important function in the codebase. Keep it pure.

**Example:**
```typescript
// Source: ARCHITECTURE.md Pattern 2

// src/diff.ts
export function computeDiff(manifest: Manifest, state: InstalledState): Diff {
  const installedSources = new Set(state.bySource.keys())

  const toAdd = manifest.sources.filter(
    source => !installedSources.has(normalizeSource(source))
  )

  const unchanged = manifest.sources.filter(
    source => installedSources.has(normalizeSource(source))
  )

  // toRemove is Phase 2 — orphan detection
  // Phase 1: always return empty toRemove
  return { toAdd, toRemove: [], unchanged }
}

function normalizeSource(source: string): string {
  // Strip trailing slashes, normalize case for comparison only
  return source.trim().toLowerCase().replace(/\/$/, '')
}
```

### Pattern 3: Executor as Dry-Run Boundary

**What:** executor.ts is the ONLY module that performs subprocess invocation. `--dry-run` is checked inside the executor before calling `execa` — all other modules are unaware of dry-run.

**When to use:** Always. Centralizing I/O side effects makes dry-run, testing, and verbose logging trivial.

**Example:**
```typescript
// Source: ARCHITECTURE.md Pattern 3

// src/executor.ts
import { execa } from 'execa'
import type { SyncOptions } from './types.js'
import { reporter } from './reporter.js'

export async function addSource(source: string, opts: SyncOptions): Promise<void> {
  const args = ['skills', 'add', source, '--yes']   // --yes always passed per locked decision
  if (opts.scope === 'global') args.push('--global')

  // Always show command being run (becomes --verbose only in Phase 2)
  reporter.printCommand(`npx ${args.join(' ')}`)

  if (opts.dryRun) return   // dry-run boundary — only exit point

  try {
    await execa('npx', args, {
      stdout: 'inherit',   // passthrough so user sees npx skills output
      stderr: 'inherit',   // CRITICAL: never swallow stderr
    })
  } catch (err) {
    // Per locked decision: warn and continue with remaining sources
    reporter.warnSourceFailed(source, err)
  }
}
```

### Pattern 4: Lock File Version Guard

**What:** Read lock file, check schema version against `EXPECTED_LOCK_VERSION` constant, throw descriptive error on mismatch rather than silently misreading state.

**When to use:** Always — first thing done when reading the lock file. Build from day one.

**Example:**
```typescript
// Source: PRD-v2.md "stable at version 3" + PITFALLS.md Pitfall 1

// src/state.ts
const EXPECTED_LOCK_VERSION = 3

interface LockFile {
  version: number
  skills: Record<string, {
    source: string
    provider: string
    url: string
    subpath?: string
  }>
}

export async function readGlobalState(): Promise<InstalledState> {
  const lockPath = path.join(os.homedir(), '.agents', '.skill-lock.json')

  let raw: string
  try {
    raw = await fs.readFile(lockPath, 'utf-8')
  } catch {
    // Missing lock file = empty state (new install, nothing installed yet)
    return { bySource: new Map() }
  }

  let lock: LockFile
  try {
    lock = JSON.parse(raw) as LockFile
  } catch {
    // Corrupt JSON = treat as empty state with warning
    reporter.warnLockFileCorrupt(lockPath)
    return { bySource: new Map() }
  }

  if (lock.version !== EXPECTED_LOCK_VERSION) {
    // Warn and fall back to empty state — do NOT throw (per locked decision)
    reporter.warnLockVersionMismatch(lock.version, EXPECTED_LOCK_VERSION)
    return { bySource: new Map() }
  }

  // Group installed skills by source for diff computation
  const bySource = new Map<string, string[]>()
  for (const [skillName, info] of Object.entries(lock.skills)) {
    const source = normalizeSource(info.source)
    const existing = bySource.get(source) ?? []
    bySource.set(source, [...existing, skillName])
  }
  return { bySource }
}
```

### Pattern 5: Config Discovery with Explicit Error

**What:** Manifest discovery follows locked priority order and fails with a specific hint when nothing is found.

**When to use:** Always — manifest.ts is responsible for the entire discovery chain.

**Example:**
```typescript
// Source: CONTEXT.md Manifest discovery order + PRD-v2.md Discovery Order

// src/manifest.ts
import { parse } from 'smol-toml'

export async function readManifest(configPath: string | undefined, scope: 'global' | 'local'): Promise<Manifest> {
  const resolvedPath = resolveManifestPath(configPath, scope)
  const content = await readManifestFile(resolvedPath)
  return parseManifest(content, resolvedPath)
}

function resolveManifestPath(configPath: string | undefined, scope: 'global' | 'local'): string {
  if (configPath) return configPath   // --config <PATH> always wins

  if (scope === 'local') {
    return path.join(process.cwd(), '.skills.toml')
  }
  // --global scope
  return path.join(os.homedir(), '.config', 'skills-sync', 'skills.toml')
}

async function readManifestFile(filePath: string): Promise<string> {
  try {
    return await fs.readFile(filePath, 'utf-8')
  } catch {
    throw new ManifestNotFoundError(filePath)
    // ManifestNotFoundError.hint = "Run 'npx skills-sync --init' to create one"
  }
}

function parseManifest(content: string, filePath: string): Manifest {
  let parsed: unknown
  try {
    parsed = parse(content)  // smol-toml throws with line/column on syntax errors
  } catch (err) {
    throw new ManifestParseError(filePath, err)
    // ManifestParseError includes the original error message with line/column
  }
  // Validate structure: must have [skills] table with repos array
  return validateManifest(parsed, filePath)
}
```

### Pattern 6: Typed Errors with Remediation Hints

**What:** Custom error classes for each failure mode, each carrying a machine-readable `hint` string shown below the error message.

**When to use:** Every user-facing error. No raw `throw new Error('...')` — always a typed error class.

**Example:**
```typescript
// Source: CONTEXT.md error design + PITFALLS.md UX section

// src/errors.ts
export abstract class SkillsSyncError extends Error {
  abstract readonly hint: string
}

export class ManifestNotFoundError extends SkillsSyncError {
  readonly hint = "Run 'npx skills-sync --init' to create one"
  constructor(filePath: string) {
    super(`No manifest found at: ${filePath}`)
  }
}

export class ManifestParseError extends SkillsSyncError {
  readonly hint: string
  constructor(filePath: string, cause: unknown) {
    const msg = cause instanceof Error ? cause.message : String(cause)
    super(`Invalid TOML in ${filePath}: ${msg}`)
    this.hint = 'Check the line/column in the error above and fix the syntax'
  }
}

export class SkillsUnavailableError extends SkillsSyncError {
  readonly hint = "Install it with: npm install -g skills"
  constructor() {
    super("'npx skills' command is unavailable")
  }
}

// In bin/skills-sync.ts top-level error handler:
// if (err instanceof SkillsSyncError) {
//   reporter.printError(err.message, err.hint)
//   process.exit(1)
// }
```

### Anti-Patterns to Avoid

- **Threading SyncOptions through non-executor modules:** ManifestReader, StateReader, and DiffComputer should not receive dryRun/verbose flags. Only Executor and Reporter need SyncOptions.
- **Writing to `~/.agents/.skill-lock.json`:** skills-sync is read-only on this file. Never write — even to "help" by updating state after installs.
- **Per-skill tracking instead of per-source tracking:** Track at source level always. A monorepo source produces N skills; tracking skill names breaks when names change between installs.
- **Reimplementing what `npx skills` does:** No GitHub API, no skill detection, no git operations. If `npx skills` can't do something, file an upstream issue.
- **Passing source strings as shell-interpolated strings:** Always use execa array API (`execa('npx', ['skills', 'add', source])`), never string concatenation into a shell command.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML parsing | Custom TOML parser | smol-toml 1.6.0 | TOML has complex edge cases (multiline strings, inline tables, dates); smol-toml is TOML 1.1.0 compliant and maintained |
| CLI argument parsing | Custom argv parser | commander 14.0.3 | Mutual exclusion, help generation, version flag, type coercion — all done and tested at 500M weekly downloads |
| Terminal colors | ANSI escape string concatenation | picocolors 1.1.1 | NO_COLOR env var, TTY detection, Windows compatibility — all handled |
| Subprocess invocation | `child_process.exec` or `spawnSync` | execa 9.6.1 | Error propagation, streaming stdio, cross-platform, array API prevents shell injection |
| TypeScript bundling | tsc alone or Rollup | tsup 8.5.1 | Dual CJS/ESM output from one command, shebang preservation, `.d.ts` generation — zero config |

**Key insight:** The entire Phase 1 value is in the orchestration logic (manifest → state → diff → execute). Every dependency problem above is solved by established libraries — spending time on custom solutions here is waste that delays the real work.

---

## Common Pitfalls

### Pitfall 1: Lock File Format Change (Undocumented API)
**What goes wrong:** Vercel can change `~/.agents/.skill-lock.json` structure without semver notice. Without a version check, skills-sync silently misreads state and produces wrong diffs (removing or reinstalling skills on every run).
**Why it happens:** The lock file is an internal implementation detail of `npx skills`, not a documented public API. It has already been "wiped" at least once between versions.
**How to avoid:** Declare `const EXPECTED_LOCK_VERSION = 3` at the top of state.ts. Check `lock.version === EXPECTED_LOCK_VERSION` before parsing skills map. On mismatch: warn and return empty state (treat as "everything needs installing" — conservative and safe).
**Warning signs:** Diffs producing "orphan" skills that are clearly still installed; users reporting skills reinstalled on every run; lock file version field does not match constant.

### Pitfall 2: Project-Scope Installs Invisible to Global Lock File
**What goes wrong:** `~/.agents/.skill-lock.json` tracks only globally installed skills. `--local` scope installs (files written to `./[agent]/skills/`) are NOT recorded in the lock file. If state.ts reads the global lock for local scope, it will always show all local skills as "not installed".
**Why it happens:** Confirmed upstream gap — `addSkillToLock()` in the skills source only runs when `installGlobally === true`. GitHub issue #337.
**How to avoid:** state.ts must branch on scope. For `--local`: scan `.agents/skills/` and `.claude/skills/` directories via `fs.readdirSync`. Never read the global lock file for local scope determination.
**Warning signs:** `--local` sync always shows all sources as missing on every run; tests only exercise `--global` and pass while `--local` is broken.

### Pitfall 3: Subprocess Errors Silently Swallowed
**What goes wrong:** `npx skills add` fails (non-zero exit, errors on stderr) but skills-sync reports "sync complete" because stderr was captured but not re-emitted.
**Why it happens:** Wrapper tools capture stdout to parse results, accidentally dropping stderr. `npx` can also exit 0 even when the inner command failed in some npm versions.
**How to avoid:** Use `execa('npx', args, { stdout: 'inherit', stderr: 'inherit' })`. Do not capture stdout/stderr as strings unless you have a specific parsing reason. Always check exit code explicitly. The locked decision mandates explicit stderr passthrough.
**Warning signs:** Skills-sync shows "installed" but skill files do not appear on disk; no visible error when using a bad source URL.

### Pitfall 4: TTY Hang — Missing `--yes` Flag to Subprocess
**What goes wrong:** `npx skills add` shows interactive prompts (skill selection, agent selection, confirmation). When spawned as a subprocess with piped stdio, these prompts hang indefinitely — especially visible in CI.
**Why it happens:** `process.stdout.isTTY` is false in subprocess context; interactive libraries wait for stdin that never comes.
**How to avoid:** Always pass `--yes` to every `npx skills add` invocation. This is a locked decision — implement from the first line of executor.ts. In Phase 1, `--agent` is not user-exposed but the executor plumbing should accept and forward it.
**Warning signs:** `npx skills-sync` hangs during sync step; CI jobs timeout with no output.

### Pitfall 5: Dry-Run Still Performs Side Effects
**What goes wrong:** `--dry-run` flag is checked at the subprocess call but earlier code still writes files or modifies state (e.g., a `.skills-sync-sources.json` write that's added later without updating the dry-run guard).
**Why it happens:** Dry-run logic is scattered across modules rather than isolated to the executor boundary.
**How to avoid:** Dry-run is checked ONCE in executor.ts, immediately before the `execa` call. No other module checks or acts on dry-run. Modules that produce side effects (future: writing `.skills-sync-sources.json`) must route through executor or an equivalent guarded path.
**Warning signs:** `--dry-run` prints "no changes would be made" but files appear or are modified on disk.

### Pitfall 6: Source Normalization Missing — Same Source Listed Twice
**What goes wrong:** Manifest has `"anthropics/claude-skills"` and lock file has source recorded as `"Anthropics/Claude-Skills"` (case difference). Without normalization, diff shows source as missing even though it's installed.
**Why it happens:** GitHub usernames and repo names are case-insensitive but not case-preserving in all contexts.
**How to avoid:** Apply `normalizeSource()` (lowercased, trimmed, trailing slash removed) consistently in both manifest.ts validation and state.ts grouping. The diff comparison always uses normalized forms.
**Warning signs:** Skills reinstalled on every sync even though they appear installed; source appears in both toAdd and unchanged on the same run.

---

## Code Examples

Verified patterns from official sources and PRD-v2.md:

### Commander 14 Setup with Mutual Exclusion
```typescript
// Source: commander 14 docs + CONTEXT.md flag design

#!/usr/bin/env node
import { Command } from 'commander'

const program = new Command()

program
  .name('skills-sync')
  .description('Declarative skill management: commit .skills.toml, run npx skills-sync')
  .version('1.0.0', '-V, --version')
  .option('--global', 'sync to global scope (~/.claude/skills etc.)')
  .option('--local', 'sync to project scope (default)')
  .option('--dry-run', 'show planned changes without executing')
  .option('--init', 'create a sample .skills.toml in current directory')
  .option('--config <path>', 'path to manifest file')

program.action(async (options) => {
  // Manual mutual exclusion for --global/--local
  if (options.global && options.local) {
    console.error('Error: --global and --local are mutually exclusive')
    process.exit(1)
  }

  const scope = options.global ? 'global' : 'local'

  if (options.init) {
    await runInit({ scope })
    process.exit(0)
  }

  await runSync({ scope, dryRun: options.dryRun ?? false, configPath: options.config })
})

program.parse()
```

### smol-toml Parsing with Line/Column Errors
```typescript
// Source: smol-toml GitHub README (https://github.com/squirrelchat/smol-toml)

import { parse, TomlError } from 'smol-toml'

function parseManifest(content: string, filePath: string): Manifest {
  let doc: unknown
  try {
    doc = parse(content)
  } catch (err) {
    if (err instanceof TomlError) {
      // smol-toml provides line and column in the error
      throw new ManifestParseError(filePath, err.message)
      // err.message already contains line:column info, e.g.:
      // "Expected newline or end of document, got ']' at line 5, column 3"
    }
    throw err
  }
  return validateManifest(doc, filePath)
}
```

### execa Subprocess with stderr Passthrough
```typescript
// Source: execa GitHub (https://github.com/sindresorhus/execa) + PITFALLS.md Pitfall 3

import { execa, ExecaError } from 'execa'

export async function addSource(source: string, opts: SyncOptions): Promise<boolean> {
  const args = ['skills', 'add', source, '--yes']
  if (opts.scope === 'global') args.push('--global')

  reporter.printAction('installing', source)

  if (opts.dryRun) {
    reporter.printDryRunCommand(`npx ${args.join(' ')}`)
    return true
  }

  try {
    await execa('npx', args, {
      stdout: 'inherit',   // show npx skills output to user
      stderr: 'inherit',   // NEVER suppress stderr
    })
    reporter.printSuccess(source)
    return true
  } catch (err) {
    if (err instanceof ExecaError) {
      reporter.warnSourceFailed(source, err.message)
    }
    return false   // per locked decision: warn and continue
  }
}
```

### Local Scope Filesystem Scan
```typescript
// Source: CONTEXT.md "For local scope" + PRD-v2.md State Reading

import * as fs from 'node:fs/promises'
import * as path from 'node:path'

export async function readLocalState(): Promise<InstalledState> {
  const scanDirs = [
    path.join(process.cwd(), '.agents', 'skills'),
    path.join(process.cwd(), '.claude', 'skills'),
  ]

  const bySource = new Map<string, string[]>()

  for (const dir of scanDirs) {
    let entries: string[]
    try {
      entries = await fs.readdir(dir)
    } catch {
      // Directory doesn't exist — that's fine, skip
      continue
    }

    for (const entry of entries) {
      // Each subdirectory is a skill name
      const skillPath = path.join(dir, entry)
      const stat = await fs.stat(skillPath).catch(() => null)
      if (stat?.isDirectory()) {
        // Local scope: source attribution not possible without .skills-sync-sources.json
        // Phase 1: no orphan detection for local — just record as "unknown source"
        const existing = bySource.get('__local__') ?? []
        bySource.set('__local__', [...existing, entry])
      }
    }
  }

  return { bySource }
}
```

### Reporter: Status Symbols and Summary
```typescript
// Source: CONTEXT.md status symbols + output format

import pc from 'picocolors'

const SYMBOLS = {
  present:     '✓',  // green
  error:       '✗',  // red
  installing:  '↓',  // cyan
  pending:     '○',  // dim
} as const

export const reporter = {
  printAction(type: 'installing', source: string) {
    console.log(`  ${pc.cyan(SYMBOLS.installing)} ${source}`)
  },

  printSuccess(source: string) {
    console.log(`  ${pc.green(SYMBOLS.present)} ${source}`)
  },

  warnSourceFailed(source: string, reason: string) {
    console.error(`  ${pc.red(SYMBOLS.error)} ${source}: ${reason}`)
  },

  printSummary(installed: number, unchanged: number) {
    // Per locked decision: "X installed, Y unchanged"
    console.log(`\n${pc.bold(`${installed} installed, ${unchanged} unchanged`)}`)
  },

  printDryRunSummary(toInstall: number, unchanged: number) {
    // Per locked decision: "X to install, Y unchanged" for dry-run
    console.log(`\n${pc.bold(`${toInstall} to install, ${unchanged} unchanged`)} ${pc.dim('(dry run)')}`)
  },

  printError(message: string, hint: string) {
    console.error(`${pc.red('Error:')} ${message}`)
    console.error(`${pc.dim('Hint:')} ${hint}`)
  },
}
```

### --init Template
```typescript
// Source: PRD-v2.md "The --init Output" + CONTEXT.md init behavior

export function generateManifestTemplate(): string {
  return `# ============================================================================
# .skills.toml - Declarative skill manifest
# ============================================================================
#
# This file declares which skills your project needs.
# Run \`npx skills-sync\` to install and sync all listed skills.
#
# USAGE:
#   npx skills-sync              Sync project skills (default)
#   npx skills-sync --global     Sync global skills
#   npx skills-sync --dry-run    Preview changes
#
# SOURCE FORMAT:
#   Each entry is passed to \`npx skills add\`, so any valid source works:
#
#   "owner/repo"                         GitHub shorthand
#   "https://github.com/owner/repo"      Full URL
#   "https://gitlab.com/group/repo"      GitLab
#   "./local/path"                       Local directory
#
# FINDING SKILLS:
#   npx skills find <query>              Search the skills registry
#   https://skills.sh                    Browse available skills
#
# ============================================================================

[skills]
repos = [
  # Add your skill sources here:
  # "owner/repo-name",
]
`
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `@iarna/toml` for TOML parsing | smol-toml 1.6.0 | @iarna/toml abandoned 2021 | smol-toml is TOML 1.1.0 compliant; @iarna/toml only reached 1.0 |
| chalk v4 for terminal color | picocolors 1.1.1 | chalk v5 went ESM-only | picocolors is 14x smaller, CJS+ESM, zero deps |
| jest for TypeScript testing | vitest 2.x | 2022+ | No babel/ts-jest config; faster; Jest-compatible API |
| `child_process.exec` for subprocess | execa 9.6.1 | 2018+ | Array API prevents shell injection; better error types |
| tsc for CLI bundling | tsup 8.5.1 | 2021+ | Dual CJS/ESM from one command; shebang preservation |

**Deprecated/outdated:**
- `@iarna/toml`: Last published 2021, TOML 0.5 only — do not use
- chalk v4: Effectively abandoned in favor of v5; do not start new projects on it
- ts-node (in tests): Slow startup; vitest handles TypeScript natively
- `child_process.exec` (for subprocess with args from user input): Shell injection risk

---

## Open Questions

1. **Exact lock file internal schema (MEDIUM confidence)**
   - What we know: PRD-v2.md states "stable at version 3" with skills map structure. DeepWiki source analysis corroborates this.
   - What's unclear: The exact field names and whether `version` is a top-level key or nested. Official Vercel docs do not document this file.
   - Recommendation: Before writing the lock file parser, read the actual `~/.agents/.skill-lock.json` from a local `npx skills` installation to verify the exact schema. Treat `EXPECTED_LOCK_VERSION = 3` as a hypothesis to confirm.

2. **Local scope diff behavior in Phase 1 (with no source attribution)**
   - What we know: Local scope scans `.agents/skills/` and `.claude/skills/` for installed skill directories. There is no `.skills-sync-sources.json` yet (that's Phase 2). Without source attribution, we can only determine if *any* skills are installed, not which source they came from.
   - What's unclear: Should Phase 1 local scope diff simply check "are there any skill directories?" or skip diff entirely and always install? Or should it always show all manifest sources as "to install" on first run?
   - Recommendation: For Phase 1 local scope, treat any skill directory as "installed but source unknown." Do NOT attempt orphan detection (that's Phase 2). Show all manifest sources as "to install" on first run (conservative approach) and install them. This matches the "missing lock file → empty state" pattern.

3. **`npx skills add` exact flag surface**
   - What we know: Official README documents `--global`, `--yes` / `-y` flags. PRD-v2.md documents `--agent` passthrough.
   - What's unclear: Whether `--agent` is `--agent=claude-code` or `--agent claude-code`; whether `--yes` is `-y` only or also `--yes`; exact error exit codes.
   - Recommendation: Verify with a real `npx skills add --help` invocation before wiring executor.ts. This is a Phase 1 implementation concern — verify before writing executor tests.

---

## Sources

### Primary (HIGH confidence)
- PRD-v2.md (`/Users/benvds/code/skills-sync/docs/PRD-v2.md`) — architecture, data flow, file format, init template, error handling table fully specified
- `.planning/research/STACK.md` — verified stack with versions, npm download counts, compatibility matrix
- `.planning/research/ARCHITECTURE.md` — component responsibilities, build order, code patterns, data flow diagrams
- `.planning/research/PITFALLS.md` — critical pitfalls with prevention strategies and recovery paths
- `.planning/research/SUMMARY.md` — executive summary of all prior research with confidence breakdown
- [smol-toml GitHub](https://github.com/squirrelchat/smol-toml) — TOML 1.1.0 parsing, v1.6.0 confirmed
- [commander GitHub releases](https://github.com/tj/commander.js/releases) — v14.0.3, Node 20+ requirement confirmed
- [execa GitHub](https://github.com/sindresorhus/execa) — v9.6.1, stdio patterns confirmed
- [picocolors GitHub](https://github.com/alexeyraspopov/picocolors) — v1.1.1, CJS+ESM confirmed
- [GitHub Issue #337](https://github.com/vercel-labs/skills/issues/337) — local scope invisible to global lock file confirmed

### Secondary (MEDIUM confidence)
- [DeepWiki: vercel-labs/skills Getting Started](https://deepwiki.com/vercel-labs/skills/2-getting-started) — lock file version 3 structure and field names
- [Node.js child_process docs](https://nodejs.org/api/child_process.html) — subprocess pitfalls
- [Node.js TTY docs](https://nodejs.org/api/tty.html) — TTY detection in subprocess context

### Tertiary (LOW confidence)
- None for Phase 1 — all critical findings are HIGH or MEDIUM confidence

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages verified via official GitHub repos and npm; version numbers confirmed; Node 20+ compatibility matrix verified
- Architecture: HIGH — PRD-v2.md fully specifies architecture; patterns corroborated by ARCHITECTURE.md derived from official Node.js docs
- Pitfalls: MEDIUM-HIGH — lock file gap confirmed via upstream GitHub issue #337; subprocess TTY behavior verified via Node.js official docs; lock file schema internals from DeepWiki (MEDIUM)
- Phase 1 scope: HIGH — CONTEXT.md decisions are locked and precise; requirements traceability complete

**Research date:** 2026-03-01
**Valid until:** 2026-04-01 (stable stack; 30-day window appropriate for established TypeScript CLI libraries)
