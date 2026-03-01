# Project Research Summary

**Project:** skills-sync v2
**Domain:** Thin TypeScript CLI — declarative skill manifest orchestration layer over `npx skills`
**Researched:** 2026-03-01
**Confidence:** HIGH

## Executive Summary

skills-sync v2 is a declarative orchestration tool — the `npm ci` to `npx skills`'s `npm install`. Its core job is to read a `.skills.toml` manifest, compare desired state to actual installed state, and delegate installs and removals to `npx skills add/remove`. The key strategic insight from research is that skills-sync must remain a thin wrapper that never reimplements what `npx skills` already does. The v1 Rust implementation failed by doing too much (custom GitHub API, custom skill detection); v2 succeeds by doing too little — manifest diffing plus subprocess delegation. The recommended stack is lean and deliberately chosen: commander 14, smol-toml, execa, picocolors, and tsup, all zero-dependency or near-zero, all actively maintained as of early 2026.

The architecture follows a linear typed pipeline: parse manifest, read actual state, compute diff (pure function), execute, report. Each stage is independently testable and has a single responsibility. The most important architectural decision — confirmed by both ARCHITECTURE.md and PITFALLS.md — is that `--local` scope installs are invisible to Vercel's global lock file (`~/.agents/.skill-lock.json`). This is a confirmed upstream gap (GitHub issue #337). skills-sync addresses it with a project-owned `.skills-sync-sources.json` tracking file, which is the correct and necessary design.

The top risks are: (1) Vercel silently changing the undocumented lock file format, requiring a schema version check at startup; (2) subprocess TTY hangs when `npx skills add` runs without `--yes` and `--agent`, requiring those flags to always be passed; and (3) swallowing subprocess errors, requiring explicit stderr passthrough via execa. All three pitfalls are mitigable in the initial implementation if the executor layer is built defensively from the start.

## Key Findings

### Recommended Stack

The stack is TypeScript 5.x on Node.js 20+ (LTS). Node 20 is a hard floor because commander v14 requires it, and v20 ships native `fetch`, reducing dependency count. The entire runtime dependency set is five packages: commander (CLI argument parsing, 500M weekly downloads, zero deps), smol-toml (TOML 1.1.0 parsing, 7M weekly downloads, zero deps, last released Dec 2025), execa (subprocess invocation without shell injection risk), picocolors (terminal color, CJS+ESM, 14x smaller than chalk), and ora (spinner during slow operations). The build toolchain is tsup + vitest — zero-config bundler for CJS/ESM dual output and a Jest-compatible test runner with native TypeScript support.

**Core technologies:**
- TypeScript 5.x: compile-time type safety catches flag/option mismatches before users hit them
- commander 14.0.3: zero-dep CLI parsing; 500M weekly downloads; mutually exclusive flag groups supported
- smol-toml 1.6.0: TOML 1.1.0 compliant; only maintained TOML parser worth using in 2026; line/column errors
- execa 9.6.1: subprocess without shell injection; explicit stdout/stderr streams; cross-platform
- picocolors 1.1.1: CJS+ESM terminal color; avoids ESM-only friction of chalk v5
- tsup 8.5.1: dual CJS/ESM output from one command; handles `bin` shebang; esbuild-powered
- vitest 2.x: native TypeScript test runner; no babel/ts-jest config needed

**Do not use:** `@iarna/toml` (unmaintained since 2021), chalk v5 alone (ESM-only complicates dual output), yargs (850 KB vs commander's 180 KB for the same surface area), oclif (12 MB for a thin wrapper), or jest (requires Babel/ts-jest for TypeScript).

### Expected Features

**Must have (table stakes):**
- TOML manifest parsing with line/column error messages — first thing users encounter
- `--init` to scaffold `.skills.toml` — every declarative tool (chezmoi, mise, pacdef) has init
- Diff computation: missing vs present vs orphaned sources — the core value loop
- Install missing skills via `npx skills add` — the sync half; always delegate, never reimplement
- Orphan detection and removal via `npx skills remove` — every package manager prunes; expected
- `--dry-run` flag — industry-wide expectation; preview before commit
- `--yes` / `-y` for CI non-interactive mode — required for automation
- `--global` / `--local` scope flags — maps to `npx skills` scope; default to `--local`
- `--config <PATH>` for custom manifest location — standard in all established CLIs
- Colored summary output with counts ("3 installed, 1 removed, 2 unchanged")
- Meaningful error messages with remediation hints

**Should have (differentiators):**
- `--check` flag: exits 1 on drift; enables CI gates ("skills match manifest" as pre-commit check)
- `.skills-sync-sources.json`: project-scope source attribution; required for local orphan detection
- Source-level tracking (not skill-level): matches how monorepos work; one source produces N skills
- `--agent` flag passthrough: prevents TTY hang; lets users target specific agents
- Pre-sync diff preview with confirmation prompt (inspired by chezmoi's most-used `diff` command)
- `--verbose` flag showing exact `npx skills` commands being run
- npm distribution via `npx skills-sync` — zero install, zero friction

**Defer indefinitely:**
- Version pinning: `npx skills` doesn't support it — would be a false promise
- Watch mode: adds background process complexity, no established user need
- Skill authoring: `npx skills init` already exists; competing adds confusion
- Interactive skill selection UI: `npx skills find` already covers this
- Custom GitHub API client: v1's critical mistake; never reimplement what upstream does

### Architecture Approach

The architecture is a linear typed pipeline with no global state. Five source modules (manifest.ts, state.ts, diff.ts, executor.ts, reporter.ts) each own one pipeline stage and pass typed data structures to the next stage. The diff computation is a pure function — no I/O, no side effects — which is the most important architectural decision because it makes the core logic trivially testable. The executor is the only module that performs subprocess I/O, making `--dry-run` trivially implementable as a check at the executor boundary without threading flags through unrelated modules.

**Major components:**
1. Entry Point (`bin/skills-sync.ts`) — thin arg parsing and routing via commander; no business logic
2. ManifestReader (`src/manifest.ts`) — locates and parses `.skills.toml`; returns typed `Manifest`
3. StateReader (`src/state.ts`) — reads global lock file OR project tracking file depending on scope; returns `InstalledState`
4. DiffComputer (`src/diff.ts`) — pure function: `(Manifest, InstalledState) => Diff`; produces `toAdd`, `toRemove`, `unchanged`
5. Executor (`src/executor.ts`) — all subprocess invocation; dry-run boundary; always passes `-y` and `--agent`
6. Reporter (`src/reporter.ts`) — colored terminal output and summary counts
7. InitCommand (`src/init.ts`) — generates sample `.skills.toml`; no dependencies

**Build order:** types.ts → errors.ts → manifest.ts → diff.ts → state.ts → reporter.ts → executor.ts → init.ts → bin/skills-sync.ts. This order allows delivering testable artifacts after each step.

### Critical Pitfalls

1. **Lock file format changes silently** — Vercel's `~/.agents/.skill-lock.json` is undocumented and has already been wiped between versions. Mitigation: read an `EXPECTED_LOCK_VERSION` constant, throw descriptive error on mismatch, fall back to `npx skills list` output parsing. Implement from day one, not retrofitted.

2. **Project-scope installs invisible in global lock** — Confirmed upstream gap (GitHub issue #337). The global lock file never records `--local` installs. Mitigation: `.skills-sync-sources.json` is the correct and essential design; diff computation must branch on scope, not rely on the global lock for local state.

3. **Subprocess errors silently swallowed** — `npx` can exit 0 even when the inner command failed. Skipping stderr re-emission causes "sync complete" when nothing was installed. Mitigation: use execa with `{ stderr: 'inherit' }` or `{ all: true }`; check exit code AND stderr presence; never merge stdout and stderr.

4. **TTY hang when `npx skills add` runs interactively** — Without `-y` and a resolved `--agent`, the subprocess hangs waiting for interactive input that never comes, causing CI timeouts. Mitigation: always pass `-y` and `--agent` when invoking `npx skills`; resolve or prompt for agent before spawning subprocess.

5. **Per-skill tracking instead of per-source tracking** — A monorepo source produces multiple skills; tracking individual skill names breaks when skill names change between installs. Mitigation: track at source level always; a source is either in the manifest or it isn't.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Foundation — Core Sync Pipeline

**Rationale:** Build the typed pipeline skeleton and the most-testable modules first. The pure diff function is the core IP; building it early with test fixtures validates the fundamental logic before any subprocess work. Architecture research explicitly recommends this build order (types → manifest → diff → state). Getting the diff right before wiring execution prevents expensive rework.

**Delivers:** A working end-to-end sync for `--global` scope: reads `.skills.toml`, reads lock file, computes diff, shells out to `npx skills add`, prints summary. Includes `--init`, `--dry-run`, `--global`/`--local` scope flags, and colored output.

**Features addressed (from FEATURES.md MVP Phase 1):**
- TOML manifest parsing with validation
- `--init` scaffold generation
- Global state reading (Vercel lock file)
- Diff computation (missing sources)
- Shell out to `npx skills add`
- `--dry-run` flag
- `--global` / `--local` scope flags
- Basic colored summary output

**Pitfalls to avoid:** Build lock file schema version check from day one (Pitfall 1). Implement executor with explicit stderr passthrough from day one (Pitfall 3). Pass `-y` and `--agent` in all subprocess calls from day one (Pitfall 4).

### Phase 2: Orphan Detection + Project Scope + Polish

**Rationale:** Orphan detection and project-scope tracking depend on the Phase 1 diff pipeline being correct. The `.skills-sync-sources.json` design is the essential mitigation for the confirmed upstream gap (project scope invisible to global lock). This cannot be deferred because it's architectural — adding it after the fact would require reworking state reading. Phase 2 also adds CI-critical flags (`--yes`, `--verbose`, `--agent`) and the pre-sync confirmation UX.

**Delivers:** Full orphan detection for both scopes, project-scope source attribution, `--yes` CI flag, `--verbose` flag, `--agent` passthrough, and pre-sync diff preview with confirmation prompt.

**Features addressed (from FEATURES.md Phase 2):**
- Orphan detection and `npx skills remove`
- `.skills-sync-sources.json` for project-scope tracking
- `--yes` flag (CI non-interactive)
- `--verbose` flag
- `--agent` flag passthrough
- Pre-sync diff preview with confirmation

**Pitfalls to avoid:** Scope branching in diff computation (Pitfall 2 — global lock vs project tracking file). Source-level tracking not skill-level tracking (Pitfall 5). Dry-run must not write `.skills-sync-sources.json` (PITFALLS.md "Looks Done But Isn't" checklist).

### Phase 3: CI Gate + Distribution

**Rationale:** `--check` flag is a differentiator (not table stakes) and depends on a correct diff pipeline from Phases 1 and 2. Distribution via npm is the final step that makes the tool available to users. Publishing before the core sync is solid risks a bad first impression.

**Delivers:** `--check` flag with correct exit codes (exits 1 on any drift including orphans), npm package publication, and README documentation with CI integration examples.

**Features addressed (from FEATURES.md Phase 3):**
- `--check` flag (CI gate, exit code 1 on drift)
- Exit code documentation for CI integration
- npm publish and README

**Pitfalls to avoid:** `--check` must exit 1 (not just print warning) for any out-of-sync condition (PITFALLS.md UX section). Include npx stale cache documentation (Pitfall 4 mitigation is documentation, not code).

### Phase Ordering Rationale

- **Dependency order:** The diff pipeline (Phase 1) must exist before orphan detection (Phase 2) can branch on scope. The full sync (Phase 2) must be correct before the CI gate (Phase 3) can be trusted.
- **Risk-first ordering:** The highest-risk pitfalls (lock file schema, TTY hang, subprocess errors) all live in the Phase 1 components. Addressing them first prevents propagating bad patterns into Phase 2 and 3.
- **Deliverable at each phase:** Phase 1 delivers a useful tool for the install case. Phase 2 adds the cleanup story. Phase 3 enables team workflows. Each phase is independently useful.
- **Anti-feature enforcement:** The decision to never reimplement `npx skills` capabilities is enforced structurally by the thin executor design. This must be established in Phase 1 to prevent scope creep in later phases.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2 (project-scope tracking):** The `.skills-sync-sources.json` schema design needs to be specified precisely before implementation. The upstream gap (GitHub issue #337) means there is no reference implementation to follow. Research should clarify: what happens when the tracking file is missing, corrupted, or out of sync with reality? What triggers a `--repair` flow?
- **Phase 3 (npm distribution):** Dual CJS/ESM output via tsup is well understood, but the `npx skills-sync` invocation path (ensuring users get the CJS entrypoint via `"bin"` field) should be verified against real-world `npx` behavior before publishing.

Phases with standard patterns (skip research-phase):
- **Phase 1 (core pipeline):** TypeScript CLI pipeline with commander, smol-toml, and execa is thoroughly documented. The architecture is specified precisely in PRD v2. The build order is clear. Standard patterns apply — no additional research needed.
- **Phase 3 (CI gate):** Exit code conventions and `--check` pattern are well-documented across analogues (deck, Argo CD, Drift). Implementation is straightforward.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All packages verified via official GitHub repos and npm; version numbers confirmed; compatibility matrix verified against Node 20+ constraint |
| Features | MEDIUM-HIGH | Table stakes verified against multiple analogues (chezmoi, pacdef, npm prune, deck); `npx skills` official docs verified; some CI gate rationale from lower-confidence single sources |
| Architecture | HIGH | PRD v2 fully specifies architecture; patterns corroborated by official Node.js docs; build order derived directly from dependency graph |
| Pitfalls | MEDIUM-HIGH | Lock file gap confirmed via upstream GitHub issue #337; subprocess TTY behavior verified via Node.js official docs; npx cache issue confirmed via npm/cli issues; lock file format mutability is MEDIUM (confirmed but format details from DeepWiki not official docs) |

**Overall confidence:** HIGH

### Gaps to Address

- **Lock file internal schema:** The exact structure of `~/.agents/.skill-lock.json` is not officially documented. Research used DeepWiki source analysis (MEDIUM confidence). During Phase 1 implementation, read the actual lock file from a local `npx skills` installation to verify the schema before writing the parsing code. Treat the `EXPECTED_LOCK_VERSION` constant as a first-class implementation concern.
- **Project tracking file schema:** `.skills-sync-sources.json` is a new file with no upstream reference. The schema (what fields, what version marker, how to handle migration) needs to be specified explicitly before Phase 2 implementation begins.
- **`npx skills` flag surface:** The exact flags accepted by `npx skills add` and `npx skills remove` (especially `--agent` values and `--global` behavior) were verified from the official README but may differ in practice. Verify with a real invocation before wiring into executor.ts.
- **Parallel subprocess execution:** Sequential `npx skills add` calls may be too slow for users with 10+ sources. Whether `npx skills` supports concurrent invocations is not confirmed. Flag this for Phase 2 or post-v1 investigation.

## Sources

### Primary (HIGH confidence)
- [vercel-labs/skills GitHub README](https://github.com/vercel-labs/skills/blob/main/README.md) — `npx skills` commands, flags, lock file location
- [GitHub Issue #337: project-scoped installs not tracked](https://github.com/vercel-labs/skills/issues/337) — confirmed upstream gap for local scope
- [commander GitHub releases](https://github.com/tj/commander.js/releases) — v14.0.3 confirmed; Node 20+ requirement
- [smol-toml GitHub](https://github.com/squirrelchat/smol-toml) — v1.6.0; TOML 1.1.0 compliance
- [execa GitHub](https://github.com/sindresorhus/execa) — v9.6.1; streaming, TTY, cross-platform
- [picocolors GitHub](https://github.com/alexeyraspopov/picocolors) — v1.1.1; CJS+ESM confirmed
- [ora npm](https://www.npmjs.com/package/ora) — v9.3.0; ESM-only; Node 18+
- [tsup npm](https://www.npmjs.com/package/tsup) — v8.5.1
- [Node.js TTY docs](https://nodejs.org/api/tty.html) — TTY detection in child processes
- [Node.js child_process docs](https://nodejs.org/api/child_process.html) — subprocess pitfalls
- [npx stale cache issue](https://github.com/npm/cli/issues/4108) — cache behavior confirmed
- [CLI Guidelines (clig.dev)](https://clig.dev/) — stderr/stdout separation best practices
- [chezmoi command overview](https://www.chezmoi.io/user-guide/command-overview/) — diff/apply/status pattern
- [npm-prune docs](https://docs.npmjs.com/cli/v9/commands/npm-prune/) — orphan pruning pattern
- PRD v2 (`/Users/benvds/code/skills-sync/docs/PRD-v2.md`) — architecture fully specified

### Secondary (MEDIUM confidence)
- [DeepWiki: vercel-labs/skills Getting Started](https://deepwiki.com/vercel-labs/skills/2-getting-started) — lock file structure internals
- [smol-toml npm weekly downloads](https://www.npmjs.com/package/smol-toml) — ~7M weekly downloads
- [Building CLI apps with TypeScript in 2026](https://dev.to/hongminhee/building-cli-apps-with-typescript-in-2026-5c9d) — Optique as emerging alternative; layered architecture patterns
- [Comparison of terminal color libraries](https://dev.to/webdiscus/comparison-of-nodejs-libraries-to-colorize-text-in-terminal-4j3a) — picocolors vs chalk benchmark
- [GitHub - Kong/deck](https://github.com/Kong/deck) — `--non-zero-exit-code` CI gate pattern
- [mise comparison to asdf](https://mise.jdx.dev/dev-tools/comparison-to-asdf.html) — declarative tool version management UX
- [GitHub - steven-omaha/pacdef](https://github.com/steven-omaha/pacdef) — `package clean`, `package sync` patterns
- [Package Manager Design Tradeoffs - Andrew Nesbitt](https://nesbitt.io/2025/12/05/package-manager-tradeoffs.html) — design decisions
- [TypeScript ESM/CJS in 2025 - Liran Tal](https://lirantal.com/blog/typescript-in-2025-with-esm-and-cjs-npm-publishing) — dual output complexity

### Tertiary (LOW confidence)
- [UX patterns for CLI tools](https://www.lucasfcosta.com/blog/ux-patterns-cli-tools) — interactive vs non-interactive mode patterns; single blog source
- [Drift Detection CI Integration](https://github.com/dadbodgeoff/drift/wiki/CI-Integration) — `--check` CI gate rationale; single source

---
*Research completed: 2026-03-01*
*Ready for roadmap: yes*
