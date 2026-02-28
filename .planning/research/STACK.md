# Stack Research

**Domain:** Rust CLI tool — GitHub API sync, TOML config, interactive terminal UX
**Researched:** 2026-02-28
**Confidence:** HIGH (all core libraries verified via official docs; versions confirmed)

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Rust (stable) | 1.84+ | Language | Single-binary distribution, no runtime, cross-platform. 1.84 adds MSRV-aware resolver which avoids dependency version toil. |
| clap | 4.5.60 | CLI argument parsing, subcommands, flag generation | De-facto standard. Derive macros eliminate boilerplate. Built-in help, error messages, shell completion hooks. No serious alternative. |
| serde | 1.0.228 | Serialization/deserialization framework | Zero-cost, compile-time, trait-based. Required by every data-touching crate. Use with `derive` feature for `#[derive(Serialize, Deserialize)]`. |
| toml | 1.0.3 (spec 1.1.0) | TOML config file parsing | Official Rust TOML crate, serde-compatible. TOML 1.1.0 spec support. Needed for `.skills.toml` manifest parsing. |
| serde_json | 1.0.149 | JSON parsing for GitHub API responses and `.skills-sync-meta.json` | The serde JSON backend. Use for deserializing GitHub Trees API responses and serializing/reading per-skill metadata files. |
| reqwest | 0.13.2 | HTTP client for GitHub API calls | Async-first, builds on tokio. Has `json` feature flag for typed serde deserialization in one call. Use over octocrab because the GitHub Trees API endpoint is not fully covered by octocrab's typed API. |
| tokio | 1.49.0 | Async runtime | Required by reqwest. De-facto async runtime for Rust; async-std was discontinued March 2025. Use `features = ["full"]` for CLI simplicity unless binary size is critical. |
| anyhow | 1.0.102 | Application-level error handling | Correct choice for application code (vs thiserror for library code). `?` propagation, `.context("...")` for user-readable error chains, `bail!` macro for early returns. Standard choice for CLI apps. |
| indicatif | 0.18.4 | Progress bars and spinners | Thread-safe, auto-hides in non-terminal environments (safe for piped output), supports multi-progress bars. Use for repo fetch and sync operations. |
| dialoguer | 0.12.0 | Interactive prompts (confirmations, selections) | Provides `Confirm`, `Select`, `MultiSelect`, `Input`. Use for orphan removal prompts and `--force` bypass. |
| dirs | 6.0.0 | Platform-aware path resolution | Provides `home_dir()`, `config_dir()`. Use for resolving `~/.claude/skills` (global target) and `~/.config/skills-sync/skills.toml` (global config). Handles Linux XDG, Windows Known Folders, macOS Standard Directories. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| thiserror | 2.0.18 | Custom error type definitions | If you extract any reusable module into a library crate later. Not needed for the initial application binary — use `anyhow` exclusively for the CLI binary. |
| owo-colors | 4.2.3 | Terminal color output | When adding colored status output (installed/updated/orphaned markers). The only actively-maintained zero-allocation color crate that handles NO_COLOR env var correctly. Use over termcolor (targets deprecated Windows Console APIs) and colored. |
| tokio (process feature) | 1.49.0 | Async subprocess management for `git clone` | When invoking `git clone --depth 1` and sparse checkout commands. Use `tokio::process::Command` (not `std::process::Command`) to avoid blocking the async runtime. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| cargo-dist | 0.30.x | Cross-platform binary release automation | Generates GitHub Actions CI that builds x86_64 Linux, x86_64 macOS, aarch64 macOS binaries and creates GitHub Releases. Also handles Homebrew tap formula generation. Run `cargo dist init` to scaffold. |
| cargo-release | latest | Version bumping and tag pushing | Works alongside cargo-dist. Handles Cargo.toml version, CHANGELOG heading, and git tag in one command. |
| clippy | bundled | Lints | Run `cargo clippy -- -D warnings` in CI to enforce quality. |
| rustfmt | bundled | Formatting | Enforce with `cargo fmt --check` in CI. |

## Installation

```toml
# Cargo.toml — [dependencies]
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
toml = "1.0"
serde_json = "1.0"
reqwest = { version = "0.13", features = ["json"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
indicatif = "0.18"
dialoguer = "0.12"
dirs = "6.0"
owo-colors = "4.2"
```

```bash
# Dev install (Homebrew tap, cargo-dist managed)
brew install <tap>/skills-sync

# Or via cargo
cargo install skills-sync
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| reqwest (direct) | octocrab | Use octocrab if you need typed GitHub-specific operations (issues, PRs, Actions). For Trees API + raw JSON, reqwest + serde_json is simpler and more direct — octocrab's typed API doesn't cover git trees. |
| tokio::process::Command for git | git2 crate | Use git2 if you need pure-Rust, no-system-git dependency. However, git2 (libgit2 bindings) has known issues with shallow clones and sparse checkout — critical features for this tool. Shelling out to system git with `tokio::process::Command` is more reliable for these operations. |
| tokio::process::Command for git | gitoxide | Gitoxide has better shallow clone support than git2 but is less mature. Would add a large transitive dependency tree. System git via subprocess is simpler and has zero compatibility risk. |
| anyhow (application) | thiserror | Use thiserror when writing library crates with structured error types callers need to match on. For this CLI binary, anyhow is correct. |
| owo-colors | termcolor | Use termcolor if you need Windows Console API compatibility for very old Windows. For modern systems, owo-colors is recommended by Rain's Rust CLI guide as the only crate meeting all criteria. |
| tokio runtime | async-std | async-std was discontinued March 2025. Do not use. |
| clap | argh, lexopt | Use argh for minimal binary size (no_std). Use lexopt for hand-rolling a minimal parser. For a real CLI with help text, color, and subcommands, clap is the only practical choice. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| async-std | Officially discontinued March 2025 | tokio |
| git2 (for shallow clone) | Known revwalk panics with shallow clones (issue #796 in rust-lang/git2-rs); sparse checkout support is unreliable | `tokio::process::Command` wrapping system `git` |
| termcolor | Targets deprecated Windows Console APIs; more complex API than needed | owo-colors |
| colored | Global state management issues; less actively maintained than owo-colors | owo-colors |
| octocrab (for Trees API) | Typed API doesn't cover `GET /repos/{owner}/{repo}/git/trees/{sha}?recursive=1`; would require falling back to raw HTTP anyway | reqwest + serde_json with typed structs |
| std::process::Command for git | Blocks the async runtime thread during clone operations | tokio::process::Command |

## Stack Patterns by Variant

**If invoking git for shallow clone:**
- Use `tokio::process::Command::new("git").args(["clone", "--depth", "1", url, dest])`
- Capture stdout/stderr, check exit code, surface errors via `anyhow::bail!`
- Avoids runtime blocking; git is universally available on target systems

**If running in `--force` mode (non-interactive):**
- Skip all `dialoguer` prompts entirely
- Use `indicatif` spinners still (useful for progress visibility without interaction)

**If `--dry-run` flag is set:**
- Do not invoke git or write any files
- Still call GitHub API to detect what would change
- Print planned operations using `owo-colors` to distinguish add/update/remove

**If writing `.skills-sync-meta.json`:**
- Use `serde_json::to_string_pretty()` for human-readable output
- Use `serde` derive on the metadata struct
- Atomic write pattern: write to `.tmp` file, then `std::fs::rename()` to avoid partial writes

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| reqwest 0.13 | tokio 1.x | reqwest 0.13 requires tokio ^1.0 as normal dependency |
| clap 4.5 | Rust 1.74+ | clap 4.x requires Rust 1.74 MSRV |
| toml 1.0 | serde 1.0 | toml 1.x requires serde with `derive` feature for struct deserialization |
| dirs 6.0 | Rust stable | Major version bump from dirs 5.x; verify API if migrating existing code |
| indicatif 0.18 | tokio 1.x | Works with async; use `ProgressBar::finish_and_clear()` to clean up properly |
| dialoguer 0.12 | console crate | dialoguer uses `console` crate internally; do not import console separately to avoid version conflicts |

## Sources

- https://docs.rs/clap/latest/clap/ — clap 4.5.60, derive macros, subcommand patterns (HIGH confidence)
- https://docs.rs/reqwest/latest/reqwest/ — reqwest 0.13.2, feature flags, json feature (HIGH confidence)
- https://docs.rs/tokio/latest/tokio/ — tokio 1.49.0, process module, full feature flag (HIGH confidence)
- https://docs.rs/anyhow/latest/anyhow/ — anyhow 1.0.102, context trait, bail macro (HIGH confidence)
- https://docs.rs/indicatif/latest/indicatif/ — indicatif 0.18.4, MultiProgress, spinner auto-tick (HIGH confidence)
- https://docs.rs/dialoguer/latest/dialoguer/ — dialoguer 0.12.0, Confirm, Select, MultiSelect (HIGH confidence)
- https://docs.rs/toml/latest/toml/ — toml 1.0.3 (TOML spec 1.1.0), serde integration (HIGH confidence)
- https://docs.rs/serde/latest/serde/ — serde 1.0.228, derive macros (HIGH confidence)
- https://docs.rs/dirs/latest/dirs/ — dirs 6.0.0, home_dir, config_dir (HIGH confidence)
- https://docs.rs/thiserror/latest/thiserror/ — thiserror 2.0.18, library vs application use (HIGH confidence)
- https://docs.rs/octocrab/latest/octocrab/ — octocrab 0.49.5, git trees API gap confirmed (HIGH confidence)
- https://rust-cli-recommendations.sunshowers.io/managing-colors-in-rust.html — owo-colors recommendation over termcolor (MEDIUM confidence, authoritative community guide)
- https://rust-cli-recommendations.sunshowers.io/handling-arguments.html — clap derive pattern recommendation (MEDIUM confidence, authoritative community guide)
- WebSearch: async-std discontinuation March 2025 confirmed by multiple sources (MEDIUM confidence)
- WebSearch: git2 shallow clone issues (issue #796, gitoxide comparison) — (MEDIUM confidence, verified against GitHub issue tracker)

---
*Stack research for: Rust CLI tool — GitHub API sync with TOML config and interactive terminal UX*
*Researched: 2026-02-28*
