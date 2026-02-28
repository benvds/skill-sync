# Architecture Research

**Domain:** Rust CLI tool — declarative package sync from GitHub
**Researched:** 2026-02-28
**Confidence:** MEDIUM-HIGH (patterns drawn from official Rust CLI book, clap maintainer blog, real Rust tools like Dotter, and verified reqwest/git documentation)

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        CLI Entry Layer                           │
│   src/main.rs — parses args, builds Context, calls orchestrator  │
│   src/cli.rs  — clap structs (Cli, Args, Flags)                  │
└─────────────────────────────┬────────────────────────────────────┘
                              │ Context struct
┌─────────────────────────────▼────────────────────────────────────┐
│                     Orchestration Layer                           │
│   src/commands/sync.rs    — full sync flow                        │
│   src/commands/init.rs    — generate sample .skills.toml          │
└──────┬────────────────────────┬───────────────────────────────────┘
       │                        │
┌──────▼──────┐        ┌────────▼────────┐
│  Config     │        │  GitHub Client  │
│  Layer      │        │  Layer          │
│             │        │                 │
│ manifest.rs │        │ github.rs       │
│ discovery   │        │ Trees API calls │
│ .skills.toml│        │ HEAD SHA fetch  │
└──────┬──────┘        └────────┬────────┘
       │                        │
┌──────▼────────────────────────▼────────────────────────────────┐
│                     Core Logic Layer                             │
│  src/resolver.rs  — diff desired vs actual state                 │
│  src/installer.rs — git subprocess: clone, pull, sparse checkout │
│  src/state.rs     — read/write .skills-sync-meta.json per skill  │
│  src/orphan.rs    — detect + prompt for removal                  │
└──────────────────────────────────────────────────────────────────┘
       │
┌──────▼────────────────────────────────────────────────────────┐
│                     Filesystem Layer                            │
│  ~/.claude/skills/          (global target)                     │
│  .claude/skills/            (local target)                      │
│  ~/.config/skills-sync/     (global config)                     │
│  .skills-sync-meta.json     (per-skill state file)              │
└─────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| `cli.rs` | Define all CLI surface: flags, args, subcommand-style behavior | `clap` derive macros on `Cli` struct |
| `main.rs` | Entry point: parse args, build `Context`, dispatch to command | Thin orchestrator, `anyhow::Result<()>` return |
| `config/manifest.rs` | Parse `.skills.toml`, validate repo list, discover file location | `toml` + `serde` deserialization |
| `github.rs` | Call GitHub Trees API, fetch default-branch HEAD SHA | `reqwest::blocking::Client` with `Authorization` header |
| `resolver.rs` | Compare desired state (manifest) against actual state (meta files); produce a plan (install/update/orphan) | Pure data transformation, no I/O |
| `installer.rs` | Execute git operations: shallow clone, fetch, sparse checkout | `std::process::Command` spawning `git` binary |
| `state.rs` | Read and write `.skills-sync-meta.json` per installed skill | `serde_json` with typed `SkillMeta` struct |
| `orphan.rs` | Detect installed skills not in manifest; prompt user | `dialoguer` Confirm prompt |
| `output.rs` | Progress bars, status messages, verbose/quiet gating | `indicatif` ProgressBar + `tracing` or simple `eprintln!` |

## Recommended Project Structure

```
skills-sync/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs             # Entry point: parse, build Context, dispatch
│   ├── cli.rs              # clap structs — Cli, Flags, target resolution
│   ├── context.rs          # Context struct (normalized config + flags)
│   ├── config/
│   │   ├── mod.rs
│   │   ├── manifest.rs     # .skills.toml parsing via serde
│   │   └── discovery.rs    # Locate config: CWD, global, --config path
│   ├── github.rs           # GitHub Trees API + HEAD SHA fetching
│   ├── resolver.rs         # Diff desired vs actual → SyncPlan
│   ├── installer.rs        # git subprocess: clone/fetch/sparse-checkout
│   ├── state.rs            # Read/write .skills-sync-meta.json
│   ├── orphan.rs           # Orphan detection + interactive removal
│   ├── output.rs           # Progress, verbosity, dry-run formatting
│   └── error.rs            # Domain error types (thiserror)
├── tests/
│   ├── manifest_parsing.rs
│   ├── resolver.rs
│   └── state.rs
└── examples/
    └── sample.skills.toml  # Used by --init output
```

### Structure Rationale

- **`context.rs`:** Prevents functions from needing to understand conflicting flags. All callers receive a normalized `Context` — recommended by Kevin K (clap maintainer) as the single most important pattern for maintainable CLI tools.
- **`config/`:** Separated from business logic because config discovery (CWD vs global vs `--config`) has its own concerns. Keeps `manifest.rs` focused purely on deserialization.
- **`resolver.rs`:** Pure function with no I/O — takes desired state + actual state, returns `SyncPlan`. Easy to test, easy to `--dry-run` (just print the plan, skip install).
- **`installer.rs`:** All git subprocess calls in one place. If switching from `std::process::Command` to another git backend, only this module changes.
- **`state.rs`:** Isolated read/write for `.skills-sync-meta.json`. Orphan detection and update checks both depend on this layer.
- **`error.rs`:** `thiserror` for domain errors (e.g., `ManifestParseError`, `GitHubApiError`); `anyhow` in `main.rs` for application-level context chaining.

## Architectural Patterns

### Pattern 1: Context Struct (Normalized Configuration)

**What:** Parse CLI args, load config file, apply defaults — then collapse everything into a single `Context` struct passed to all business logic.

**When to use:** Always. Multi-source config (CLI flags + TOML + defaults) is the defining complexity in any sync tool.

**Trade-offs:** Small upfront cost to define `Context`; saves significant complexity downstream as features grow.

**Example:**
```rust
// cli.rs — raw parse result
#[derive(Parser)]
pub struct Cli {
    #[arg(long)] pub dry_run: bool,
    #[arg(long)] pub force: bool,
    #[arg(long)] pub verbose: bool,
    #[arg(long)] pub config: Option<PathBuf>,
    #[arg(long, default_value = "local")]
    pub target: TargetScope,
}

// context.rs — normalized
pub struct Context {
    pub manifest: Manifest,
    pub skills_dir: PathBuf,    // resolved: local or global
    pub dry_run: bool,
    pub force: bool,
    pub verbose: bool,
}

impl Context {
    pub fn from_cli(cli: &Cli) -> anyhow::Result<Self> {
        let manifest_path = discover_manifest(cli.config.as_deref())?;
        let manifest = parse_manifest(&manifest_path)?;
        let skills_dir = resolve_skills_dir(cli.target);
        Ok(Context { manifest, skills_dir, dry_run: cli.dry_run, force: cli.force, verbose: cli.verbose })
    }
}
```

### Pattern 2: Plan/Execute Split (Resolver → Installer)

**What:** Separate the "what needs to change" step from the "make the change" step. `resolver.rs` produces a `SyncPlan`; `installer.rs` executes it.

**When to use:** Always in sync/package tools. Enables `--dry-run` for free, simplifies testing (resolver is pure), and decouples GitHub API calls from git operations.

**Trade-offs:** One extra data type (`SyncPlan`); worth it for testability and `--dry-run` support.

**Example:**
```rust
// resolver.rs
pub struct SyncPlan {
    pub to_install: Vec<SkillSpec>,
    pub to_update: Vec<SkillSpec>,   // local SHA != remote HEAD SHA
    pub orphaned: Vec<InstalledSkill>,
}

pub fn resolve(manifest: &Manifest, installed: &[InstalledSkill]) -> SyncPlan {
    // pure: no I/O, no side effects
    // diff manifest.skills vs installed by repo URL
}
```

### Pattern 3: Per-Item Metadata Files (State Tracking)

**What:** Write a `.skills-sync-meta.json` file alongside each installed skill. Each file contains the repo URL, installed SHA, skill name, and timestamp.

**When to use:** When tracking state for a set of independently-installed items where orphan detection and selective update are required.

**Trade-offs:** More files on disk vs a central manifest. Advantage: each skill is self-describing; orphan detection is a simple filesystem scan that doesn't require a lock on a central state file.

**Example:**
```rust
// state.rs
#[derive(Serialize, Deserialize)]
pub struct SkillMeta {
    pub repo: String,
    pub sha: String,
    pub skill_name: String,
    pub installed_at: String,  // ISO 8601
}

pub fn write_meta(skills_dir: &Path, skill_name: &str, meta: &SkillMeta) -> anyhow::Result<()> {
    let path = skills_dir.join(skill_name).join(".skills-sync-meta.json");
    let json = serde_json::to_string_pretty(meta)?;
    fs::write(path, json)?;
    Ok(())
}
```

### Pattern 4: git Subprocess over libgit2

**What:** Call the system `git` binary via `std::process::Command` for clone and fetch operations, rather than using the `git2` crate (libgit2 bindings).

**When to use:** Whenever shallow clones (`--depth 1`) or sparse checkout are required. libgit2 does not support shallow clones; it silently ignores the depth parameter.

**Trade-offs:** Requires `git` to be installed on the user's system (safe assumption for developers). Avoids large libgit2 build dependency.

**Example:**
```rust
// installer.rs
pub fn shallow_clone(repo_url: &str, target_dir: &Path) -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["clone", "--depth", "1", repo_url, target_dir.to_str().unwrap()])
        .status()
        .context("Failed to spawn git")?;
    if !status.success() {
        anyhow::bail!("git clone failed for {}", repo_url);
    }
    Ok(())
}
```

## Data Flow

### Primary Sync Flow

```
User: `skills-sync`
    │
    ▼
cli.rs: parse Cli struct (clap)
    │
    ▼
context.rs: Context::from_cli()
    │   ├─ discovery.rs: find .skills.toml
    │   ├─ manifest.rs: parse + validate TOML
    │   └─ resolve skills_dir (local or global)
    │
    ▼
commands/sync.rs: run_sync(&ctx)
    │
    ├─ github.rs: for each repo in manifest
    │   ├─ GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1
    │   │   → detect single-skill vs monorepo
    │   └─ GET /repos/{owner}/{repo}/commits/{branch}
    │       → fetch remote HEAD SHA
    │
    ├─ state.rs: scan skills_dir for installed skills
    │   → read each .skills-sync-meta.json
    │
    ├─ resolver.rs: resolve(manifest, installed_skills)
    │   → SyncPlan { to_install, to_update, orphaned }
    │
    ├─ output.rs: print plan (always)
    │   └─ if --dry-run: return here
    │
    ├─ installer.rs: for each to_install
    │   ├─ git clone --depth 1 <repo>
    │   └─ state.rs: write_meta()
    │
    ├─ installer.rs: for each to_update
    │   ├─ git -C <skill_dir> fetch --depth 1
    │   ├─ git -C <skill_dir> reset --hard FETCH_HEAD
    │   └─ state.rs: write_meta() with new SHA
    │
    └─ orphan.rs: if orphaned not empty
        ├─ dialoguer: prompt (skip if --force)
        └─ fs::remove_dir_all() for confirmed orphans
```

### Key Data Flows

1. **Manifest to Context:** `.skills.toml` (TOML) → `Manifest` struct (serde) → `Context` (normalized with paths resolved)
2. **GitHub API to SyncPlan:** Remote Trees API response → repo structure detection → HEAD SHA → compared against `SkillMeta.sha` → `SyncPlan` entries
3. **SyncPlan to Filesystem:** Install/update operations write skill files + `.skills-sync-meta.json`; orphan detection reads then deletes

### State Management (no database required)

```
Desired state:  .skills.toml (manifest)
Actual state:   .skills-sync-meta.json files (one per skill)
                + skills_dir filesystem structure

Resolver reads both → produces delta plan
Installer applies plan → updates actual state
```

## Scaling Considerations

This is a local developer CLI tool — "scaling" means handling larger manifests and more skills, not concurrent users.

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 1-20 skills | Sequential API calls and installs; synchronous reqwest::blocking is fine |
| 20-100 skills | Consider parallel GitHub API calls using rayon or tokio; rate limit awareness needed |
| 100+ skills | GitHub API rate limit (60 req/hr unauthenticated, 5000 req/hr authenticated) becomes the binding constraint; batch requests or cache aggressively |

### Scaling Priority

1. **First bottleneck:** GitHub API rate limits (60/hr unauthenticated). Solution: require `gh auth` token or `GITHUB_TOKEN` env var, add retry-after header handling.
2. **Second bottleneck:** Sequential git clones are slow with many skills. Solution: spawn parallel git processes (rayon `par_iter` over install list).

## Anti-Patterns

### Anti-Pattern 1: Embedding GitHub API calls in Resolver

**What people do:** Call the GitHub API inside resolver logic to check current state mid-resolution.

**Why it's wrong:** Makes the resolver impure and untestable. Resolver tests now require network mocking. `--dry-run` becomes entangled with real API calls.

**Do this instead:** Fetch all remote data before calling the resolver. Pass pre-fetched `RemoteSkillInfo` structs into `resolve()`. Resolver stays pure.

### Anti-Pattern 2: Central State File (Single Lockfile)

**What people do:** Store all installed skill state in a single `skills-sync-lock.json` at the skills dir root.

**Why it's wrong:** Concurrent invocations corrupt the file. Users who manually add/remove skill directories break state consistency. Orphan detection requires comparing file to directory contents.

**Do this instead:** Use per-skill `.skills-sync-meta.json` files colocated with each skill. Orphan detection is a directory scan — any skill directory without a meta file is flagged.

### Anti-Pattern 3: Using libgit2 (git2 crate) for Shallow Clones

**What people do:** Pull in the `git2` crate expecting to get full git functionality including shallow clone.

**Why it's wrong:** libgit2 silently ignores the `--depth` parameter for clones. You'll get full clones with no error message, which is slow and defeats the tool's efficiency goal.

**Do this instead:** Spawn the `git` binary directly via `std::process::Command`. Developers always have `git` available. This also handles sparse checkout correctly.

### Anti-Pattern 4: Async Runtime for Simple Sequential CLI

**What people do:** Add `tokio` and make everything `async` because HTTP calls are involved.

**Why it's wrong:** For a CLI that makes a handful of sequential HTTP calls, async adds boilerplate without benefit. The reqwest docs explicitly recommend the blocking API for scripts and simple tools that don't need concurrency.

**Do this instead:** Use `reqwest::blocking::Client`. Add `tokio` only if/when parallel skill installation is added in a later phase.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| GitHub REST API | `reqwest::blocking::Client` with `User-Agent` and optional `Authorization: Bearer` headers | Rate limits: 60/hr unauth, 5000/hr auth. Trees API endpoint: `GET /repos/{owner}/{repo}/git/trees/{sha}?recursive=1` |
| git binary | `std::process::Command` spawning system `git` | Required for shallow clone (`--depth 1`) and sparse checkout. libgit2 cannot do shallow clones. |
| Filesystem | `std::fs`, `dirs` crate for home dir | Skills dirs: `.claude/skills/` (local) or `~/.claude/skills/` (global) |
| User terminal | `dialoguer` for confirmations, `indicatif` for progress | Respect `NO_COLOR`, `CI` env vars to disable interactive prompts |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `main.rs` → commands | Pass `&Context` by reference | Context is immutable after construction |
| `github.rs` → `resolver.rs` | Pass `Vec<RemoteSkillInfo>` (pre-fetched data) | Resolver never calls network — keeps it pure |
| `resolver.rs` → `installer.rs` | Pass `SyncPlan` struct | Plan is the contract between "what to do" and "how to do it" |
| `installer.rs` → `state.rs` | `write_meta()` called after each successful install/update | State only written on success — partial failures leave previous state intact |
| `state.rs` → `orphan.rs` | `Vec<InstalledSkill>` read from disk | Orphan detection is a set subtraction: installed minus manifest |

## Build Order Implications

Components should be built in this order due to data dependencies:

1. **`error.rs` + `state.rs`** — foundational types used everywhere
2. **`config/`** — manifest parsing before any logic is possible
3. **`github.rs`** — needed to test resolver with real data shapes
4. **`resolver.rs`** — pure logic, easiest to test once types exist
5. **`installer.rs`** — git subprocess, testable with temp directories
6. **`cli.rs` + `context.rs`** — wire the above together
7. **`commands/sync.rs`** — orchestrates all layers
8. **`output.rs` + `orphan.rs`** — UX layer added last

This order means you can test resolver correctness with mocked data before any git operations exist, and add UX polish (progress bars, prompts) last without blocking core functionality.

## Sources

- [Command Line Applications in Rust (official book)](https://rust-cli.github.io/book/index.html) — HIGH confidence
- [Kevin K's Blog: CLI Structure in Rust](https://kbknapp.dev/cli-structure-01/) — HIGH confidence (clap crate maintainer)
- [Cargo Package Layout — official docs](https://doc.rust-lang.org/cargo/guide/project-layout.html) — HIGH confidence
- [reqwest::blocking docs](https://docs.rs/reqwest/latest/reqwest/blocking/index.html) — HIGH confidence
- [libgit2 shallow clone issue #2782](https://github.com/libgit2/libgit2/issues/2782) — HIGH confidence (confirms libgit2 limitation)
- [Leapcell: Rust Project Organization](https://leapcell.io/blog/how-to-organize-a-large-scale-rust-project-effectively) — MEDIUM confidence
- [Dotter dotfiles manager (Rust)](https://github.com/SuperCuber/dotter) — MEDIUM confidence (real-world reference implementation for sync tools in Rust)
- [Rust error handling: anyhow + thiserror pattern](https://leapcell.io/blog/choosing-the-right-rust-error-handling-tool) — MEDIUM confidence

---
*Architecture research for: Rust CLI skill sync tool managing external GitHub packages*
*Researched: 2026-02-28*
