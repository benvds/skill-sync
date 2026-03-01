# Feature Landscape

**Domain:** Declarative CLI skill/package management tool (thin wrapper over `npx skills`)
**Researched:** 2026-03-01
**Confidence:** MEDIUM-HIGH (Vercel skills CLI verified via official GitHub README; package manager patterns cross-verified across multiple established tools)

---

## Context

skills-sync sits in a specific niche: it is not a package manager, and it is not a skill installer. It is a **declarative orchestration layer** — the `npm ci` to `npx skills`'s `npm install`. The feature set must be evaluated against that role.

Analogues studied: Nix, chezmoi, pacdef, mise/asdf (declarative tool management), npm/pip/cargo (package manifests), deck (Kong config drift detection), Argo CD (GitOps drift detection). These inform table stakes expectations for users who have worked with any declarative sync tool.

---

## Table Stakes

Features users expect. Missing = product feels incomplete or untrustworthy.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| TOML manifest parsing and validation | Every declarative tool starts with reading a config file. Users expect clear error messages on syntax errors with line/column info. | Low | Use `smol-toml` or `@iarna/toml`. Parser should report line numbers. |
| `--init` to scaffold manifest | All major declarative tools (chezmoi, mise, pacdef) have an init command. Users need a starting point. | Low | Generate `.skills.toml` with documented comments. Include usage instructions inline. |
| Diff computation: missing vs present vs orphaned | This is the core value loop. Without showing what changed, users can't trust the tool. chezmoi, pacdef, Argo CD all show diff before apply. | Medium | Compare manifest sources against lock file (`~/.agents/.skill-lock.json`) and filesystem. |
| Install missing skills via `npx skills add` | The sync half of sync. Delegates to Vercel's CLI — skills-sync shells out, does not reimplement. | Medium | Pass `--yes`, `--global`, `--agent` flags through as appropriate. |
| Orphan detection and removal via `npx skills remove` | Every declarative package manager removes unmanaged packages (pacdef's `package clean`, npm's `npm prune`, Argo CD's pruning). Users expect this. | Medium | Orphan = installed skill whose source is not in manifest. Prompt before removal unless `--yes`. |
| `--dry-run` flag | Industry-wide expectation. chezmoi has `-n`, kluctl has `--dry-run`, deck has `--non-zero-exit-code`. Users preview before committing changes. | Low | Print planned actions, take no action. Must be clear about what would happen. |
| `--yes` / `-y` flag for CI | Non-interactive mode is expected by any tool running in automation. Pacdef has `--noconfirm`, npm has `--yes`. | Low | Pass `-y` to all `npx skills` subcommands. Skip internal confirmation prompts. |
| `--global` / `--local` scope flags | `npx skills` has `-g`/`--global`. All multi-scope tools (npm, chezmoi, mise) distinguish global vs project scope. | Low | Mutually exclusive. Default to `--local`. Map to corresponding `npx skills` flags. |
| `--config <PATH>` for custom manifest location | Standard in all established CLIs (pip, cargo, chezmoi). | Low | Override default discovery order. |
| Colored summary output | Users expect human-readable feedback: what was installed, removed, skipped. chezmoi, pacdef, npm all use colored terminal output. | Low | Show counts: X installed, Y removed, Z unchanged. Use ANSI colors. |
| Meaningful error messages | Missing manifest → hint to run `--init`. Invalid TOML → line/column. `npx skills` unavailable → install hint. | Low | Every error must suggest a remedy. |
| `--verbose` flag showing delegated commands | Users debugging issues need to see the exact `npx skills` commands being run. mise and chezmoi both show underlying ops on verbose. | Low | Print each `npx skills add/remove ...` command before executing it. |

---

## Differentiators

Features that set skills-sync apart. Not expected, but meaningfully valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| `--check` flag (CI gate, exit code 1 if out of sync) | No analog in `npx skills`. decK added `--non-zero-exit-code` to their diff command and it became a top feature for CI pipelines. Drift is a tool whose entire purpose is CI gates. Lets teams enforce "skills match manifest" as a pre-commit or PR check. | Low | Read-only. Exits 0 if in sync, 1 if drift detected. Print drift summary. Does not install or remove anything. |
| `.skills-sync-sources.json` for project-scope source attribution | `npx skills` lock file only covers global installs. Without project-scope tracking, orphan detection for `--local` is impossible. This fills a gap in the ecosystem. | Medium | Written by skills-sync, never by `npx skills`. Maps source → installed skill names for the project scope. |
| Source-level tracking (not skill-level) | A monorepo source like `anthropics/claude-skills` produces multiple skills. Tracking at source level is the correct abstraction — matches how users think about their manifest entries. | Medium | One entry in `.skills.toml` may produce N installed skills. Diff at source granularity. |
| `--agent` flag passthrough | Users targeting a specific agent (e.g., only Claude Code) without touching others. `npx skills` supports this natively; skills-sync surfaces it cleanly. | Low | Pass `--agent <agent>` to `npx skills add`. Document which agents `npx skills` supports. |
| Human-readable pre-sync diff preview | Even without `--dry-run`, show a quick diff before executing. chezmoi's `diff` command is one of its most-used features. Users want to confirm before committing. | Low | Print planned changes (N to install, M to remove) and ask for confirmation unless `--yes`. |
| Clear summary report with counts | `Done. 3 installed, 1 removed, 2 unchanged.` — simple, scannable, CI-friendly. Many tools omit this or make it verbose. | Low | Always print summary line. Never silent success. |
| npm distribution via `npx skills-sync` | Zero install. Users don't configure anything — `npx skills-sync` just works. Aligns with how `npx skills` itself is distributed. | Low | Already decided in PROJECT.md. Critical for adoption: reduces friction to zero. |

---

## Anti-Features

Features to explicitly NOT build.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Custom GitHub API client | This was v1's mistake. `npx skills` handles all GitHub interaction. Reimplementing creates maintenance burden and feature lag. | Delegate entirely to `npx skills add <source>` |
| Custom skill detection (SKILL.md traversal) | `npx skills` already handles monorepo detection, subdirectory traversal, multi-source. Reimplementing it means bugs and drift from the canonical implementation. | Let `npx skills add` do detection |
| Writing to Vercel's `~/.agents/.skill-lock.json` | This is Vercel's file. Writing it outside of `npx skills` would create state corruption and break the ecosystem's single source of truth. | Read it, never write it |
| Version pinning / SHA pinning | `npx skills` doesn't support it. Building it in skills-sync would create a false promise — users could pin a manifest entry but `npx skills add` would still install latest. | Document as a future consideration if `skills` adds support |
| Watch mode / auto-sync | Adds complexity, creates background processes, surprising behavior. No established declarative tool in this domain has watch mode as a core feature. | Users run `npx skills-sync` explicitly, or add it to CI |
| Skill authoring / scaffolding | `npx skills init` already exists. Competing would add confusion. | Point users to `npx skills init` in docs |
| Interactive skill selection UI (fzf-style) | `npx skills find` already provides this. Reimplementing would be feature creep. | Delegate to `npx skills find` |
| Private repo authentication configuration | Beyond scope of a thin wrapper. What `npx skills` supports for auth is sufficient. | Document that auth flows through `npx skills` |
| Dependency resolution between skills | Skills don't have dependencies on each other. Adding this would be building a feature nobody asked for. | N/A |
| Multi-level manifest inheritance / workspace mode | Overly complex for a tool aiming for <1000 LOC. Solve the simple case first. | Single `.skills.toml` per project |

---

## Feature Dependencies

```
--init
  └─ (no dependencies — creates the manifest)

--dry-run
  └─ Diff computation (requires: manifest parsing + actual state reading)

--check
  └─ Diff computation (same engine as --dry-run, just exits non-zero instead of listing)
  └─ Manifest parsing
  └─ Actual state reading (lock file + filesystem)

sync (default)
  └─ Manifest parsing
  └─ Actual state reading
  └─ Diff computation
  └─ npx skills add (for installs)
  └─ npx skills remove (for orphan removal)
  └─ .skills-sync-sources.json read/write (for project-scope orphan detection)

Orphan detection (--local scope)
  └─ .skills-sync-sources.json (required — Vercel lock file does not cover project scope)

Orphan detection (--global scope)
  └─ ~/.agents/.skill-lock.json (Vercel's lock file, read-only)

--yes
  └─ Passed through to: confirmation prompts + npx skills subcommands

--verbose
  └─ Surfaces the exact npx skills commands being executed

--agent
  └─ Passed through to: npx skills add

--global / --local
  └─ Determines: which lock file / filesystem paths to read
  └─ Determines: which npx skills flags to pass (-g or not)
```

---

## MVP Recommendation

Prioritize (Phase 1 — Core Sync):
1. TOML manifest parsing with validation and clear errors
2. `--init` scaffold generation
3. Actual state reading (Vercel lock file for `--global`, filesystem for `--local`)
4. Diff computation (missing sources)
5. Shell out to `npx skills add` for installs
6. `--dry-run` flag
7. `--global` / `--local` scope flags
8. Basic colored summary output

Prioritize (Phase 2 — Orphan Detection + Polish):
1. Orphan detection and `npx skills remove` for cleanup
2. `.skills-sync-sources.json` for project-scope source attribution
3. `--yes` flag (CI non-interactive)
4. `--verbose` flag showing delegated commands
5. `--agent` flag passthrough
6. Pre-sync diff preview with confirmation prompt

Prioritize (Phase 3 — Team Workflow):
1. `--check` flag (CI gate, exit code 1 on drift)
2. Human-readable exit code documentation for CI integration
3. npm publish and README documentation

Defer indefinitely:
- Version pinning: `npx skills` doesn't support it, false promise
- Watch mode: adds complexity, no clear user need established
- Skill authoring: `npx skills init` already covers this

---

## Sources

- [GitHub - vercel-labs/skills README](https://github.com/vercel-labs/skills/blob/main/README.md) — HIGH confidence: official source for `npx skills` commands and flags
- [Vercel Knowledge Base: Agent Skills](https://vercel.com/kb/guide/agent-skills-creating-installing-and-sharing-reusable-agent-context) — HIGH confidence: official Vercel documentation
- [chezmoi Command Overview](https://www.chezmoi.io/user-guide/command-overview/) — HIGH confidence: official chezmoi docs; informed diff/apply/status pattern
- [GitHub - Kong/deck](https://github.com/Kong/deck) — MEDIUM confidence: `--non-zero-exit-code` pattern for CI gate informed `--check` flag design
- [Comparison to asdf - mise](https://mise.jdx.dev/dev-tools/comparison-to-asdf.html) — MEDIUM confidence: declarative tool version management UX patterns
- [GitHub - steven-omaha/pacdef](https://github.com/steven-omaha/pacdef) — MEDIUM confidence: `package clean` (orphan removal), `package sync` patterns
- [npm-prune documentation](https://docs.npmjs.com/cli/v9/commands/npm-prune/) — HIGH confidence: official npm docs; orphan pruning pattern
- [UX patterns for CLI tools](https://www.lucasfcosta.com/blog/ux-patterns-cli-tools) — LOW confidence: single source, community blog; interactive vs non-interactive mode patterns
- [Package Manager Design Tradeoffs - Andrew Nesbitt](https://nesbitt.io/2025/12/05/package-manager-tradeoffs.html) — MEDIUM confidence: recent analysis of package manager design decisions
- [Drift Detection CI Integration](https://github.com/dadbodgeoff/drift/wiki/CI-Integration) — LOW confidence: single source; informed `--check` flag CI gate rationale
