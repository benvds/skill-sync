# Project Research Summary

**Project:** skills-sync
**Domain:** Rust CLI tool — declarative GitHub-sourced agentic skill sync
**Researched:** 2026-02-28
**Confidence:** HIGH

## Executive Summary

skills-sync is a declarative package manager for Claude Code skills (SKILL.md convention) sourced from GitHub repositories. Research confirms this is a well-understood domain with clear precedent: tools like lazy.nvim (Neovim plugins), chezmoi (dotfiles), and the `sk`/`aps` CLI competitors all solve the same "desired state from remote source → local filesystem" problem. The recommended approach is a Rust single binary with a manifest-driven sync loop: parse `.skills.toml`, call GitHub Trees API to detect repo structure, compare local SHA metadata against remote HEAD, and execute shallow clones or sparse checkouts as needed. The plan/execute split (resolver → installer) is the defining pattern — it enables `--dry-run` for free and keeps business logic testable without network or filesystem mocking.

The Rust stack is fully validated with no meaningful alternatives to evaluate: clap for CLI, reqwest (blocking) for GitHub API, tokio only if parallel installs are added later, serde/toml for config, serde_json for metadata and API responses, anyhow for error handling, indicatif for progress, and dialoguer for interactive orphan prompts. The `git` binary (subprocess) is the correct approach for shallow clones and sparse checkout — libgit2 silently ignores `--depth 1` and has unreliable sparse checkout support, making it the single most dangerous technology trap in this domain. Distribution via cargo-dist covers Homebrew tap + GitHub Releases binary automation.

The primary risks are all in the GitHub API and git operation layers: unauthenticated requests hit a 60 req/hr rate limit; the Trees API silently truncates responses for large repos without an error; missing User-Agent headers return opaque 403s; and shallow clone + sparse checkout in combination has subtle interaction bugs on update operations. The secondary risk is state corruption: metadata files must be written after (not before) a successful clone, using atomic rename, and SIGINT during git operations must trigger cleanup rather than leaving partial directories. Address all six critical pitfalls before shipping any sync functionality — they are not polish, they are correctness requirements.

## Key Findings

### Recommended Stack

The stack is dominated by the Rust async ecosystem and official crates with no viable alternatives in most slots. reqwest should use the blocking API (not async) for v1 — the reqwest docs explicitly recommend this for simple sequential CLI tools. Add tokio only when parallel installation becomes a roadmap item. The single build dependency worth noting: `dialoguer` uses the `console` crate internally — do not import `console` separately or version conflicts will occur.

**Core technologies:**
- Rust 1.84+ — single binary, no runtime, cross-platform; 1.84 adds MSRV-aware resolver
- clap 4.5 (derive) — CLI parsing, subcommands, help generation; de-facto standard with no real alternative
- serde 1.0 + toml 1.0 — TOML manifest deserialization; official crate with serde compatibility
- serde_json 1.0 — GitHub API response parsing and `.skills-sync-meta.json` read/write
- reqwest 0.13 (blocking + json features) — GitHub REST API calls; direct over octocrab because Trees API is not covered by octocrab's typed API
- anyhow 1.0 — application-level error propagation with `.context()` chains; correct choice for binary crate (vs thiserror for library crates)
- indicatif 0.18 — progress bars/spinners; auto-hides in non-terminal environments
- dialoguer 0.12 — orphan removal confirmation prompt; `Confirm`, `Select` primitives
- dirs 6.0 — platform-aware path resolution for `~/.claude/skills` and `~/.config/skills-sync/`
- owo-colors 4.2 — terminal color output; handles `NO_COLOR` env var correctly
- cargo-dist 0.30.x — cross-platform binary release automation and Homebrew tap generation

**Explicit exclusions:**
- async-std: discontinued March 2025
- git2 crate: silently ignores `--depth 1`; unreliable sparse checkout
- octocrab: typed API does not cover git trees endpoint
- tokio (v1): add only when parallel installs are added

### Expected Features

Research identified 16 table stakes features that all competitors implement, plus 8 differentiators where skills-sync can exceed the field. The full prioritized list is in FEATURES.md.

**Must have (table stakes):**
- Declarative `.skills.toml` manifest — users think in "desired state", not commands
- `sync` command (idempotent) — install missing, update outdated, leave current alone
- Install missing skills via shallow clone (`--depth 1`) or sparse checkout for monorepos
- Update outdated skills via SHA comparison (local metadata SHA vs remote HEAD)
- Orphan detection with interactive prompt for removal
- State tracking via `.skills-sync-meta.json` per skill — prerequisite for update and orphan detection
- `--dry-run`, `--force`, `--verbose` flags — CLIG standard; all competitors have these
- `--init` flag — generate commented sample `.skills.toml`; critical for adoption
- Local and global install targets (`.claude/skills/` and `~/.claude/skills/`)
- Graceful error handling — network failures must not corrupt state
- Exit codes 0/non-zero — required for script integration
- Single-skill repo detection and monorepo detection via GitHub Trees API
- Manifest discovery: CWD walk-up then global config fallback (same as git config)

**Should have (competitive differentiators):**
- Rust single binary with zero runtime deps — primary differentiator over Python/Node/Bun competitors
- SHA-based update detection via API before cloning — avoids unnecessary clone operations
- Sparse checkout for monorepo skill extraction — lighter than full clone
- Verbose inline TOML comments in `--init` output — config as tutorial
- Color-coded per-skill output (green = installed, yellow = updated, red = error)

**Defer (v2+):**
- Version/branch pinning — no stable skill versioning convention exists yet
- Lock file for reproducible installs — conflicts with "always latest" v1 model
- Non-GitHub sources (GitLab, Bitbucket) — doubles API surface; ecosystem is GitHub-native
- Watch mode — daemon complexity for infrequent operation
- Skill dependency resolution — no established convention yet
- `--json` output — add if CI integration demand emerges

### Architecture Approach

The architecture follows the plan/execute split as its central organizing principle: a pure `resolver.rs` produces a `SyncPlan` from desired state (manifest) and actual state (metadata files), and `installer.rs` executes it. This keeps the resolver testable without network mocking and gives `--dry-run` support for free — just print the plan and return. All runtime config (CLI flags + TOML + defaults) collapses into a single `Context` struct before any business logic runs, eliminating flag-conflict bugs and preventing "what does --force mean here?" questions from propagating into every function signature.

**Major components:**
1. `cli.rs` + `context.rs` — clap argument parsing and normalization into `Context`; all flag resolution happens here
2. `config/manifest.rs` + `config/discovery.rs` — TOML deserialization and CWD/global config file location
3. `github.rs` — GitHub Trees API structure detection and HEAD SHA fetching; data is pre-fetched before resolver runs
4. `resolver.rs` — pure function: desired state + actual state → `SyncPlan`; no I/O, no network calls
5. `installer.rs` — git subprocess execution: shallow clone, fetch+reset, sparse checkout
6. `state.rs` — read/write `.skills-sync-meta.json` per skill; written only after successful git operation
7. `orphan.rs` — set subtraction (installed minus manifest); dialoguer confirmation prompt
8. `output.rs` — indicatif progress bars, owo-colors status, verbose/dry-run gating

**Build order dictated by data dependencies:**
`error.rs` + `state.rs` → `config/` → `github.rs` → `resolver.rs` → `installer.rs` → `cli.rs` + `context.rs` → `commands/sync.rs` → `output.rs` + `orphan.rs`

### Critical Pitfalls

1. **GitHub Trees API `truncated: true` field not checked** — silent data loss for large repos; always check `response.truncated` before parsing tree, fall back to non-recursive traversal if true. This is a required correctness check, not a nice-to-have.

2. **Unauthenticated API calls hit 60 req/hr rate limit** — breaks users with 10+ repos or on shared networks; check for `GITHUB_TOKEN` env var on startup, fall back to `gh auth token` subprocess, warn prominently if neither is available. Authenticate every request.

3. **Missing User-Agent header returns opaque 403** — reqwest does not set one automatically; set `User-Agent: skills-sync/{version}` on the `reqwest::Client` builder, not per-request.

4. **`.skills-sync-meta.json` written before git clone completes** — SIGINT or network failure between the two leaves state corrupted; next run sees "up to date" for a broken skill. Write metadata only after successful git operation, using atomic `write-to-temp + rename()` pattern.

5. **Shallow clone + sparse checkout interaction on update** — update fetch on a shallow-sparse clone can reset sparse-checkout patterns and include unexpected files; use `git clone --no-checkout --filter=blob:none --depth 1` + `git sparse-checkout set --cone <dir>` + `git checkout` for install; re-apply cone patterns explicitly on update.

6. **No SIGINT handling leaves partial clone directories** — register a SIGINT handler (ctrlc crate) or write a `.skills-sync-partial` sentinel file at install start; clean up partial directories on next startup scan.

## Implications for Roadmap

Based on architecture build-order dependencies, pitfall phase assignments, and feature dependencies from FEATURES.md, the recommended phase structure is:

### Phase 1: Project Bootstrap and Core Types

**Rationale:** Foundational types (`SkillMeta`, `SkillSpec`, `SyncPlan`, `ManifestEntry`, error types) must exist before any other module can be written. This phase is pure Rust with no external dependencies to debug.
**Delivers:** Compiling crate skeleton, `Cargo.toml` with all dependencies pinned, `error.rs` domain types, `state.rs` read/write with atomic rename, `config/manifest.rs` TOML parsing
**Addresses:** Table stakes — manifest parsing, state tracking metadata
**Avoids:** Pitfall 4 (metadata write ordering) — design the atomic write pattern here before installer is written

### Phase 2: GitHub API Integration

**Rationale:** All business logic depends on remote data shapes. The resolver and installer cannot be designed correctly until GitHub API response types are defined and validated. Auth and rate-limit handling must be the first thing built in this module — adding them later is the most common cause of production incidents in this domain.
**Delivers:** Authenticated `reqwest::blocking::Client`, Trees API call with truncation check, HEAD SHA fetch, `RemoteSkillInfo` typed response struct, rate-limit header reading
**Addresses:** Trees API structure detection, SHA-based update detection
**Avoids:** Pitfall 1 (truncated field), Pitfall 2 (rate limits), Pitfall 3 (User-Agent)

### Phase 3: Resolver and Sync Plan

**Rationale:** Pure logic with no I/O — the easiest phase to test thoroughly. With real GitHub API types defined from Phase 2, the resolver can be built and unit-tested with mocked data before any git operations exist. This is the heart of the tool.
**Delivers:** `resolver.rs` — pure `resolve(manifest, installed, remote_info) -> SyncPlan`; unit tests covering install/update/orphan/up-to-date cases
**Addresses:** Idempotent sync semantics, orphan detection logic, update detection
**Uses:** SyncPlan struct (Phase 1), RemoteSkillInfo (Phase 2), SkillMeta (Phase 1)

### Phase 4: Git Operations and Installer

**Rationale:** git subprocess integration is the highest-risk technical component. The shallow+sparse checkout combination (Pitfall 4) requires deliberate design of both install and update flows together before coding either. Build against a local bare repo in tests to avoid network dependency.
**Delivers:** `installer.rs` — shallow clone, fetch+reset for updates, sparse checkout for monorepos; post-install `state.rs` write; SIGINT sentinel file pattern
**Addresses:** Install missing skills, update outdated skills, monorepo sparse checkout
**Avoids:** Pitfall 4 (shallow + sparse interaction), Pitfall 5 (metadata write timing), Pitfall 6 (SIGINT partial state)

### Phase 5: CLI Surface and Context Normalization

**Rationale:** With all business logic working, wire it together through the CLI. Building `Context` after the business modules are stable means the normalization layer reflects real needs rather than guesses.
**Delivers:** `cli.rs` clap struct, `context.rs` normalization, `config/discovery.rs` CWD walk-up + global fallback, `commands/sync.rs` orchestration, `commands/init.rs` template generation
**Addresses:** `--dry-run`, `--force`, `--verbose`, `--init` flags; manifest discovery; local vs global target
**Implements:** Context struct pattern (resolves flag conflicts), discovery chain

### Phase 6: UX Polish and Orphan Flow

**Rationale:** UX layer (progress bars, color output, orphan prompts) is decoupled from correctness. Adding it last means correctness is validated before UX complexity is introduced.
**Delivers:** `output.rs` indicatif progress + owo-colors status, `orphan.rs` dialoguer confirmation, `--force` bypass of prompt, structured summary output ("3 up to date, 1 installed, 0 orphaned"), error message wrapping (no raw git stderr)
**Addresses:** UX pitfalls — progress feedback, contextual error messages, orphan dismissal instructions
**Avoids:** UX pitfalls from PITFALLS.md (raw git stderr, no progress, silent success)

### Phase 7: Distribution and Release Automation

**Rationale:** Final phase packages the working tool for distribution. cargo-dist automation means Homebrew tap and GitHub Releases are low-effort once the binary is stable.
**Delivers:** cargo-dist configuration, GitHub Actions CI for x86_64 Linux/macOS and aarch64 macOS, Homebrew tap formula, `cargo install` support, clippy + rustfmt CI gates
**Addresses:** Distribution differentiator (Rust binary vs runtime-dependent competitors)

### Phase Ordering Rationale

- Phases 1→2→3 follow the architecture build-order from ARCHITECTURE.md: types before API, API before resolver
- Phase 4 is isolated from Phase 3 because git operations are independently testable once SyncPlan types exist
- Phase 5 is deferred until business logic is stable — clap structs are cheapest to write once requirements are known
- Phase 6 last: PITFALLS.md explicitly maps UX pitfalls to "CLI UX layer" phase; they do not block correctness
- Phase 7 is naturally last; no business logic dependencies

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 4 (Git Operations):** The shallow clone + sparse checkout interaction (Pitfall 4) is poorly documented in combination. PITFALLS.md outlines the correct command sequence but it should be validated against a real test repo before finalizing the implementation plan. Consider a spike task.
- **Phase 5 (Config Discovery):** CWD walk-up with global fallback has edge cases (symlinks, mounted paths, Windows path separators). Worth reviewing chezmoi's discovery implementation as a reference before writing `discovery.rs`.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Bootstrap):** Standard Rust crate setup; no novel patterns
- **Phase 2 (GitHub API):** reqwest + serde JSON is well-documented; pitfalls are known and preventable
- **Phase 3 (Resolver):** Pure data transformation; no external dependencies or novel patterns
- **Phase 6 (UX):** indicatif and dialoguer have extensive examples; patterns are clear
- **Phase 7 (Distribution):** cargo-dist has detailed documentation and scaffolding command

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All crates verified against official docs.rs; versions confirmed; alternatives evaluated with specific reasons |
| Features | MEDIUM-HIGH | Table stakes and differentiators confirmed against 4 direct competitors via repo inspection; some competitor features not fully documented |
| Architecture | MEDIUM-HIGH | Patterns drawn from official Rust CLI book, clap maintainer blog, and Dotter (real Rust sync tool); build order validated against data dependencies |
| Pitfalls | HIGH | GitHub API limits from official docs; git2 shallow clone issue confirmed against GitHub issue tracker; sparse checkout from official git docs |

**Overall confidence:** HIGH

### Gaps to Address

- **Windows path handling:** Architecture and pitfalls research focused on POSIX systems. Use `std::path::PathBuf` throughout (never string `/` concatenation) and test on Windows before v1 release. The `dirs` crate handles Windows Known Folders but actual path separator behavior in git subprocess args needs validation.
- **`gh auth token` subprocess availability:** Auth fallback to `gh auth token` assumes GitHub CLI is installed. Research did not verify availability on CI systems. Treat as optional fallback; `GITHUB_TOKEN` env var is primary; document clearly.
- **Default branch name assumption:** Research notes storing resolved branch name in metadata to avoid per-sync API calls, but did not fully specify when to resolve it (install time vs runtime). Clarify this in Phase 2 design: resolve at install, store in SkillMeta, use stored value for update checks.
- **Monorepo one-level-deep assumption:** FEATURES.md specifies one-level-deep scan for skill subdirectories. Edge case: skills nested more than one level deep in a monorepo will be silently missed. This is a known scope decision, not a gap — document it in `--init` output and error messages.

## Sources

### Primary (HIGH confidence)
- https://docs.rs/clap/latest/clap/ — clap 4.5.60, derive macros, subcommand patterns
- https://docs.rs/reqwest/latest/reqwest/ — reqwest 0.13.2, blocking client, json feature
- https://docs.rs/tokio/latest/tokio/ — tokio 1.49.0, process module
- https://docs.rs/anyhow/latest/anyhow/ — anyhow 1.0.102, context, bail
- https://docs.rs/indicatif/latest/indicatif/ — indicatif 0.18.4
- https://docs.rs/dialoguer/latest/dialoguer/ — dialoguer 0.12.0
- https://docs.rs/toml/latest/toml/ — toml 1.0.3
- https://docs.rs/serde/latest/serde/ — serde 1.0.228
- https://docs.rs/dirs/latest/dirs/ — dirs 6.0.0
- https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api — rate limits (60/hr unauth, 5000/hr auth)
- https://docs.github.com/en/rest/git/trees — Trees API, truncated field, 100k/7MB limits
- https://docs.github.com/rest/guides/best-practices-for-using-the-rest-api — User-Agent requirement
- https://git-scm.com/docs/git-sparse-checkout — cone vs non-cone mode, O(N*M) cost
- https://rust-cli.github.io/book/index.html — Rust CLI book (signals, error handling)
- https://kbknapp.dev/cli-structure-01/ — Context struct pattern (clap maintainer)
- https://github.com/libgit2/libgit2/issues/2782 — libgit2 shallow clone limitation confirmed

### Secondary (MEDIUM confidence)
- https://clig.dev/ — CLI UX standards (dry-run, verbose, force, exit codes)
- https://rust-cli-recommendations.sunshowers.io/ — owo-colors recommendation, clap derive pattern
- https://github.com/folke/lazy.nvim — plugin manager patterns (lockfile, orphan cleanup, dry-run)
- https://github.com/westonplatter/agentic-prompt-sync — direct competitor feature set
- https://github.com/803/skills-supply — direct competitor, TOML manifest
- https://github.com/runkids/skillshare — direct competitor
- https://github.com/dhruvwill/skills-cli — direct competitor
- https://github.com/SuperCuber/dotter — Rust dotfiles sync tool reference implementation
- https://github.blog/open-source/git/bring-your-monorepo-down-to-size-with-sparse-checkout/ — sparse checkout for monorepos

### Tertiary (LOW confidence)
- https://github.com/travisvn/awesome-claude-skills — SKILL.md structure reference (incomplete)
- https://github.com/hesreallyhim/awesome-claude-code — Claude skills ecosystem structure
- async-std discontinuation (March 2025) — verified via multiple web sources, no single canonical announcement

---
*Research completed: 2026-02-28*
*Ready for roadmap: yes*
