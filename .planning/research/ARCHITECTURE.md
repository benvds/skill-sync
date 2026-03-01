# Architecture Research

**Domain:** Thin TypeScript CLI — manifest-driven declarative sync orchestrator
**Researched:** 2026-03-01
**Confidence:** HIGH (PRD fully specifies architecture; patterns corroborated by official Node.js docs and current CLI tooling literature)

## Standard Architecture

### System Overview

```
.skills.toml (desired state)
        │
        ▼
┌───────────────────────────────────────────────────────────────┐
│                        CLI Entry Point                         │
│  bin/skills-sync.ts  (shebang, parse argv, route to command)  │
└───────────────────────────┬───────────────────────────────────┘
                            │
           ┌────────────────┼─────────────────┐
           ▼                ▼                 ▼
   ┌──────────────┐  ┌─────────────┐  ┌─────────────────┐
   │  ManifestReader│  │ StateReader │  │ InitCommand     │
   │  (parse TOML) │  │(lock file + │  │(generate sample │
   │               │  │ filesystem) │  │ .skills.toml)   │
   └──────┬────────┘  └──────┬──────┘  └─────────────────┘
          │                  │
          └────────┬─────────┘
                   ▼
          ┌─────────────────┐
          │  DiffComputer   │
          │ (desired vs     │
          │  actual state)  │
          └────────┬────────┘
                   │
          ┌────────┴────────┐
          ▼                 ▼
  ┌──────────────┐  ┌──────────────┐
  │  DryRunReport│  │  Executor    │
  │  (preview)   │  │ (shell out   │
  │              │  │  npx skills) │
  └──────────────┘  └──────┬───────┘
                            │
                   ┌────────┴────────┐
                   ▼                 ▼
        npx skills add         npx skills remove
        (install missing)      (remove orphaned)

External state stores (read-only from skills-sync):
  ~/ .agents/.skill-lock.json   (global scope)
  ./.agents/skills/             (project scope, filesystem)
  ./.claude/skills/             (project scope, filesystem)

skills-sync writes:
  ./.skills-sync-sources.json   (project-scope source attribution only)
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| Entry Point (`bin/skills-sync.ts`) | Parse argv, validate flag combinations, route to correct command, set exit codes | `commander` or custom arg parsing; shebang `#!/usr/bin/env node` |
| ManifestReader | Locate `.skills.toml` (config discovery order), parse TOML, validate entries, return typed `Manifest` | `smol-toml` or `@iarna/toml`; throws on parse errors |
| StateReader | Read Vercel's lock file (global) or scan filesystem (local); return `InstalledState` keyed by source | `JSON.parse` for lock file; `fs.readdirSync` for filesystem scan |
| DiffComputer | Pure function: `(desired: string[], actual: InstalledState) => Diff`; produces `toAdd`, `toRemove`, `unchanged` lists | Zero dependencies; pure function, easily testable |
| Executor | Shell out to `npx skills add/remove` respecting `--dry-run`, `--yes`, `--agent`, `--verbose` flags | `execa` for cross-platform child process; streaming output in verbose mode |
| Reporter | Colored terminal output, progress indicators, final summary | `chalk` for color; custom print helpers |
| InitCommand | Generate sample `.skills.toml` to stdout or write to disk | Template string; check for existing file to avoid overwrite |

## Recommended Project Structure

```
skills-sync/
├── bin/
│   └── skills-sync.ts        # Entry point: shebang, parse argv, route
├── src/
│   ├── manifest.ts           # Read + validate .skills.toml
│   ├── state.ts              # Read lock file + filesystem (actual state)
│   ├── diff.ts               # Pure diff computation (desired vs actual)
│   ├── executor.ts           # Shell out to npx skills add/remove
│   ├── reporter.ts           # Terminal output, colors, summary
│   ├── init.ts               # --init command: generate sample manifest
│   ├── types.ts              # Shared TypeScript types (Manifest, InstalledState, Diff, etc.)
│   └── errors.ts             # Typed error classes with user-facing messages
├── test/
│   ├── manifest.test.ts      # Unit tests for TOML parsing and validation
│   ├── state.test.ts         # Unit tests for lock file parsing
│   ├── diff.test.ts          # Unit tests for diff logic (pure function, easy to test)
│   └── fixtures/             # Sample .skills.toml, lock file JSON for tests
├── package.json
├── tsconfig.json
└── .skills.toml              # Skills-sync eats its own dog food
```

### Structure Rationale

- **`bin/`:** Thin entry point only — contains shebang and argv wiring. No business logic. Keeps `src/` importable as a library if needed.
- **`src/manifest.ts`, `state.ts`, `diff.ts`:** Each owns one pipeline stage. Single responsibility means each file is independently testable.
- **`src/diff.ts` as pure function:** The diff computation is the core IP. Making it a pure function (no side effects, no I/O) means it's trivially unit-testable with fixture data.
- **`src/executor.ts`:** All subprocess invocation isolated here. `--dry-run` is implemented entirely in this layer — the rest of the pipeline runs normally, executor just skips the actual `execa` call and logs what would run.
- **`src/types.ts`:** Shared types prevents circular imports. `Manifest`, `InstalledState`, `Diff`, `SyncOptions` all live here.
- **`test/fixtures/`:** Real `.skills.toml` and `skill-lock.json` samples prevent tests from drifting from real-world formats.

## Architectural Patterns

### Pattern 1: Pipeline with Typed Stage Outputs

**What:** Each pipeline stage returns a typed data structure consumed by the next stage. No global state — everything is passed explicitly.

**When to use:** Always for this tool. The linear pipeline (parse → read → diff → execute → report) maps cleanly to explicit function calls.

**Trade-offs:** Slightly more verbose than threading a mutable context object, but dependencies are explicit and each stage is independently testable.

**Example:**
```typescript
// types.ts
export interface Manifest { sources: string[] }
export interface InstalledState { bySource: Map<string, string[]> }
export interface Diff {
  toAdd: string[]       // sources to install
  toRemove: string[]    // skill names to remove (orphans)
  unchanged: string[]   // sources already present
}
export interface SyncOptions {
  scope: 'global' | 'local'
  dryRun: boolean
  yes: boolean
  verbose: boolean
  agent?: string
  configPath?: string
}

// main sync flow in bin/skills-sync.ts
const manifest = await readManifest(options.configPath, options.scope)
const state    = await readActualState(options.scope)
const diff     = computeDiff(manifest, state)
await execute(diff, options)
```

### Pattern 2: Pure Diff Function

**What:** `computeDiff(manifest, state)` takes two data structures and returns a `Diff` with no I/O, no side effects, and no flags threading. All decision logic lives here.

**When to use:** Always. This is the most important function in the codebase. Keep it pure.

**Trade-offs:** Requires callers to have already gathered all state before calling — no lazy loading. Acceptable for a CLI that runs once.

**Example:**
```typescript
// diff.ts
export function computeDiff(manifest: Manifest, state: InstalledState): Diff {
  const toAdd = manifest.sources.filter(
    source => !state.bySource.has(normalizeSource(source))
  )
  const manifestSourceSet = new Set(manifest.sources.map(normalizeSource))
  const toRemove = [...state.bySource.entries()]
    .filter(([source]) => !manifestSourceSet.has(source))
    .flatMap(([, skills]) => skills)
  const unchanged = manifest.sources.filter(
    source => state.bySource.has(normalizeSource(source))
  )
  return { toAdd, toRemove, unchanged }
}
```

### Pattern 3: Executor as Dry-Run Boundary

**What:** The `executor.ts` module is the only place that performs subprocess invocation. `--dry-run` is implemented by checking the flag inside the executor before calling `execa` — every other module is unaware of dry-run.

**When to use:** Any CLI that delegates to external processes. Centralizing I/O side effects makes dry-run, testing, and verbose logging trivial to add.

**Trade-offs:** None meaningful at this scale. Avoids threading `dryRun` flags through unrelated modules.

**Example:**
```typescript
// executor.ts
async function addSource(source: string, opts: SyncOptions): Promise<void> {
  const args = ['skills', 'add', source]
  if (opts.scope === 'global') args.push('--global')
  if (opts.yes) args.push('--yes')
  if (opts.agent) args.push('--agent', opts.agent)

  if (opts.verbose || opts.dryRun) {
    console.log(`  $ npx ${args.join(' ')}`)
  }
  if (opts.dryRun) return  // stop here for dry-run

  await execa('npx', args, { stdio: opts.verbose ? 'inherit' : 'pipe' })
}
```

## Data Flow

### Primary Sync Flow

```
User runs: npx skills-sync [flags]
    │
    ▼
bin/skills-sync.ts
  Parse argv → SyncOptions
  Validate: --global/--local mutually exclusive
    │
    ▼
manifest.ts: readManifest(configPath?, scope)
  Discover .skills.toml (explicit → cwd → ~/.config/skills-sync/)
  Parse TOML → Manifest { sources: string[] }
  Validate: non-empty strings, no duplicates, trim whitespace
    │
    ▼
state.ts: readActualState(scope)
  IF scope=global:
    Read ~/.agents/.skill-lock.json
    Parse JSON → extract skills map
    Group by source → InstalledState
  IF scope=local:
    Scan .agents/skills/ + .claude/skills/
    Check .skills-sync-sources.json for source attribution
    → InstalledState
    │
    ▼
diff.ts: computeDiff(manifest, state) → Diff
  toAdd:    sources in manifest not in state
  toRemove: skills in state whose source is not in manifest
  unchanged: sources present in both
    │
    ▼
reporter.ts: printDiffSummary(diff)
  Show what will happen (always shown, before confirmation)
    │
    ▼
IF --dry-run: print plan, exit 0
IF --check:   exit 1 if diff non-empty, exit 0 if clean
ELSE:
  IF toRemove && !--yes: prompt user for orphan confirmation
    │
    ▼
executor.ts: execute(diff, options)
  For each source in toAdd:  execa('npx', ['skills', 'add', source, ...])
  For each skill in toRemove: execa('npx', ['skills', 'remove', skill, ...])
    │
    ▼
reporter.ts: printSummary(results)
  "N installed, N removed, N unchanged"
```

### Config Discovery Flow

```
--config <PATH> provided?
  YES → use that path (error if not found)
  NO  →
    scope=local?
      YES → look for .skills.toml in cwd
      NO  → look for ~/.config/skills-sync/skills.toml
    Found?
      YES → use it
      NO  → error: "Run 'npx skills-sync --init' to create one"
```

### State Attribution Flow (Project Scope)

```
.skills-sync-sources.json exists?
  YES → use source attribution from it (enables orphan detection)
  NO  → filesystem scan only; installed skills cannot be attributed to sources
        → orphan detection disabled (warn user)
```

## Scaling Considerations

This is a local CLI tool with no server component. "Scaling" means handling edge cases in the sync pipeline.

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 1-5 sources | Current architecture is fine; sequential installs |
| 10-50 sources | Consider parallel `npx skills add` invocations; add progress bar |
| Monorepo with many skills | Source-level diff already handles this correctly; no per-skill tracking needed |
| CI environment | `--yes --check` flags handle non-interactive use; `--verbose` for debug output |

### Scaling Priorities

1. **First bottleneck:** Sequential `npx skills add` calls — each may take seconds. Mitigation: parallel execution with concurrency limit (Promise.allSettled with semaphore). Not needed for MVP.
2. **Second bottleneck:** Subprocess overhead from `npx` prefix — consider checking if `skills` is globally installed and calling directly. Deferrable.

## Anti-Patterns

### Anti-Pattern 1: Threading SyncOptions Through Every Module

**What people do:** Pass `options.dryRun`, `options.verbose` etc. into `ManifestReader`, `StateReader`, and `DiffComputer`.

**Why it's wrong:** These modules do not perform I/O side effects. They shouldn't know about dry-run. It creates hidden coupling and makes unit testing require constructing full `SyncOptions` objects.

**Do this instead:** Only `Executor` and `Reporter` need `SyncOptions`. Earlier pipeline stages take only what they need (e.g., `scope`, `configPath`).

### Anti-Pattern 2: Writing to Vercel's Lock File

**What people do:** Update `~/.agents/.skill-lock.json` directly after installation to avoid re-reading the file.

**Why it's wrong:** The lock file is owned by Vercel's `skills` package. Writing to it risks format drift, corruption, and breaking the tools users depend on. skills-sync is explicitly a layer on top, not a replacement.

**Do this instead:** Re-read the lock file after execution if you need fresh state. For the sync workflow, you don't need to re-read — you know what you just installed.

### Anti-Pattern 3: Per-Skill State Tracking Instead of Per-Source

**What people do:** Track every individual installed skill name in the sync metadata.

**Why it's wrong:** A monorepo source (e.g., `anthropics/claude-skills`) produces multiple skills. If you track at the skill level, you must decide which manifest entry "owns" each skill — which breaks when skill names change between installs.

**Do this instead:** Track at the source level. A source is in the manifest or it isn't. All skills from an installed source are desired; all skills from a missing source are orphaned.

### Anti-Pattern 4: Reimplementing What `npx skills` Already Does

**What people do:** Add GitHub API calls, custom skill detection, or git operations to work around `npx skills` limitations.

**Why it's wrong:** This is the exact trap the v1 Rust implementation fell into. The strategic insight of v2 is that skills-sync's value is the declarative manifest, not the execution engine.

**Do this instead:** If `npx skills` can't do something you need, file an issue upstream. Keep skills-sync as a thin orchestration layer.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| `npx skills add` | `execa` subprocess; inherit or pipe stdio | Pass `--yes`, `--global`, `--agent` flags through; treat non-zero exit as recoverable error (warn + continue) |
| `npx skills remove` | `execa` subprocess; inherit stdio | Non-zero exit is a warning, not fatal |
| `~/.agents/.skill-lock.json` | Read-only JSON file; `JSON.parse` | Never write; treat missing file as empty state |
| `.skills-sync-sources.json` | Read + write JSON; only file skills-sync owns | Write after successful installs; schema versioned at v1 |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `bin/` to `src/` | Direct function imports | Entry point is thin; all logic in `src/` |
| `manifest.ts` to `state.ts` | No direct coupling; both feed `diff.ts` independently | They run sequentially but neither depends on the other |
| `diff.ts` to `executor.ts` | `Diff` type is the contract | Diff is pure data; executor decides how to act on it |
| `executor.ts` to `reporter.ts` | Reporter called from executor after each action | Or executor returns results and caller invokes reporter — either works; keep consistent |

## Suggested Build Order

Dependencies between components determine build order. Build in this sequence:

1. **`src/types.ts`** — All shared types. No dependencies. Everything else imports from here.
2. **`src/errors.ts`** — Error classes with user-facing messages. No dependencies.
3. **`src/manifest.ts`** — Reads and validates `.skills.toml`. Depends only on types and errors.
4. **`src/diff.ts`** — Pure diff computation. Depends only on types. Most testable module — build with tests.
5. **`src/state.ts`** — Reads lock file and filesystem. Depends on types and errors.
6. **`src/reporter.ts`** — Terminal output. Depends on types. Can be built as stubs and filled in later.
7. **`src/executor.ts`** — Subprocess invocation. Depends on types, reporter, errors. Integrates with real `npx skills`.
8. **`src/init.ts`** — `--init` command. Depends on nothing (template string output). Can be built anytime.
9. **`bin/skills-sync.ts`** — Entry point. Wires all modules together. Build last.

This order means each phase of implementation can deliver a working artifact:
- After step 4: Pure logic (manifest + diff) is testable with fixtures
- After step 5: State reading from lock file works end-to-end without subprocess
- After step 7: Full sync pipeline works
- After step 9: Distributable CLI is complete

## Sources

- PRD v2 (`/Users/benvds/code/skills-sync/docs/PRD-v2.md`) — architecture and data flow fully specified
- Node.js official docs: [Child Process](https://nodejs.org/api/child_process.html) — subprocess patterns (HIGH confidence)
- [Building CLI apps with TypeScript in 2026](https://hackers.pub/@hongminhee/2026/typescript-cli-2026) — parser-first design, layered architecture (MEDIUM confidence)
- [execa npm package](https://www.npmjs.com/package/execa/v/5.1.0) — cross-platform subprocess (HIGH confidence — widely used)
- Existing Rust v1 source (`/Users/benvds/code/skills-sync/src/`) — manifest validation patterns preserved in TypeScript rewrite

---
*Architecture research for: skills-sync v2 — thin TypeScript CLI, manifest-driven sync orchestrator*
*Researched: 2026-03-01*
