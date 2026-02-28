# Phase 1: Foundation - Research

**Researched:** 2026-02-28
**Domain:** Rust CLI crate scaffold, TOML parsing, file I/O, error handling
**Confidence:** HIGH

## Summary

Phase 1 builds a greenfield Rust CLI that parses `.skills.toml` manifests, validates entries, discovers config from multiple locations (CWD, XDG global, explicit `--config`), generates a sample config via `--init`, and reads/writes per-skill metadata atomically. No network or git operations are in scope.

The Rust ecosystem provides mature, well-tested crates for every concern: `toml` + `serde` for TOML parsing, `clap` for CLI argument parsing, `dirs` for XDG directory resolution, `tempfile` for atomic writes, `regex` for repo pattern validation, `chrono` for ISO 8601 timestamps, and `miette` + `thiserror` for Cargo-style diagnostic errors with colored output.

**Primary recommendation:** Use the standard Rust CLI stack (clap + serde + toml + miette + thiserror) with `tempfile::NamedTempFile::persist()` for atomic metadata writes. Keep modules cleanly separated: manifest, config discovery, validation, metadata, errors, init.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **--init output style:** Tutorial-style generated `.skills.toml` with full header, usage section, format explanation, and commented examples. Self-teaching for new users. Include 2-3 commented-out example repos. Global init location: `~/.config/skills-sync/skills.toml`. If `.skills.toml` already exists: error with message showing path to existing file, refuse to overwrite.
- **Error message tone:** Cargo-style structured errors: `error: manifest not found` with `-->` hints and remediation suggestions. Semantic terminal colors: red=error, yellow=warning, green=success, dim=secondary info. Respect `NO_COLOR` env var. Default output: summary only. Every error includes remediation hint.
- **Validation behavior:** Strict + helpful: reject invalid entries with specific error (line number, what's wrong, how to fix). No silent skipping. Empty repos list: warn and exit cleanly (exit 0). Whitespace/trailing slashes: auto-trim silently. Unknown TOML keys: warn and continue.
- **Metadata schema:** Include `"schema_version": 1` from day one. ISO 8601 timestamps. Fields: source, subdirectory (null for single-skill), sha, installed_at, updated_at. Meta files stay visible (not gitignored).

### Claude's Discretion
- Exact Cargo.toml dependency versions (use latest stable per research)
- Module file organization within src/
- Test structure and fixture design
- Atomic write implementation approach (temp file + rename)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| MNFST-01 | Define skill repos in `.skills.toml` using `owner/repo` format | `toml` crate with serde Deserialize for typed manifest struct |
| MNFST-02 | Validate repo entries match `^[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+$` and reject duplicates | `regex` crate for pattern validation; HashSet for duplicate detection |
| MNFST-03 | Discover manifest via CWD, `~/.config/skills-sync/skills.toml`, or `--config <path>` | `dirs::config_dir()` for XDG path; `clap` for `--config` flag |
| MNFST-04 | Generate documented sample `.skills.toml` via `--init` | String template written to CWD; existence check before write |
| SYNC-05 | Write `.skills-sync-meta.json` per skill with source, SHA, timestamps | `serde_json` for serialization; `tempfile` for atomic writes |
| ERR-01 | Missing manifest produces error with `--init` hint | `miette` diagnostic with help text |
| ERR-02 | Invalid TOML syntax produces error with line/description | `toml` crate provides span info; `miette` renders source snippets |
| ERR-05 | Exit 0 on success, non-zero on error | `std::process::ExitCode` or `std::process::exit()` |
| ERR-06 | GitHub API requests include User-Agent header | Not applicable in Phase 1 (no network); prepare User-Agent constant |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5 | CLI argument parsing with derive macros | De facto Rust CLI standard, 300M+ downloads |
| serde | 1.0 | Serialization/deserialization framework | Ubiquitous Rust serialization, 700M+ downloads |
| toml | 0.8 | TOML file parsing/writing | Official Rust TOML implementation, 400M+ downloads |
| serde_json | 1.0 | JSON serialization for metadata files | Standard JSON crate, 600M+ downloads |
| thiserror | 2.0 | Derive macro for custom error types | Standard for library-style error enums |
| miette | 7.0 | Fancy diagnostic error reporting | Cargo-style error display with source snippets |
| regex | 1.12 | Repo pattern validation | Standard regex engine, linear-time guarantees |
| dirs | 6.0 | XDG/platform config directory resolution | Cross-platform standard directories, 150M+ downloads |
| chrono | 0.4 | ISO 8601 timestamp generation | Standard Rust datetime library |
| tempfile | 3.0 | Temporary file creation for atomic writes | Standard temp file crate, supports persist() |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| owo-colors | 4.0 | Zero-allocation terminal coloring | For colored error/warning/success output |
| supports-color | 3.0 | Detect terminal color support + NO_COLOR | Gate color output on terminal capability |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| miette | anyhow | anyhow is simpler but lacks Cargo-style diagnostic rendering |
| owo-colors | colored | colored is more popular but heavier; owo-colors is zero-alloc |
| chrono | time | time crate is lighter but chrono has better ISO 8601 formatting |
| regex | manual parsing | Manual parsing is faster but error-prone for edge cases |

**Installation:**
```bash
cargo add clap --features derive
cargo add serde --features derive
cargo add toml serde_json thiserror miette --features miette/fancy
cargo add regex dirs chrono tempfile owo-colors supports-color
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── main.rs          # Entry point, CLI dispatch
├── cli.rs           # Clap argument definitions
├── config.rs        # Manifest discovery (CWD, global, --config)
├── manifest.rs      # TOML parsing + validation
├── metadata.rs      # SkillMeta struct, atomic read/write
├── init.rs          # --init command implementation
└── error.rs         # Error types with miette diagnostics
```

### Pattern 1: Clap Derive for CLI
**What:** Use `#[derive(Parser)]` for type-safe CLI argument parsing
**When to use:** Always for CLI entry point
**Example:**
```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "skills-sync", about = "Sync Claude skills from manifest")]
struct Cli {
    /// Generate a sample .skills.toml
    #[arg(long)]
    init: bool,

    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,
}
```

### Pattern 2: Serde Deserialize for Manifest
**What:** Define typed Rust structs that map to TOML structure
**When to use:** For parsing .skills.toml
**Example:**
```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    skills: SkillsSection,
}

#[derive(Deserialize)]
struct SkillsSection {
    repos: Vec<String>,
}
```

### Pattern 3: thiserror + miette for Diagnostics
**What:** Define error enum with thiserror, wrap with miette for display
**When to use:** All error types
**Example:**
```rust
use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
enum SkillsSyncError {
    #[error("manifest not found")]
    #[diagnostic(
        code(skills_sync::manifest::not_found),
        help("Run `skills-sync --init` to create a sample manifest")
    )]
    ManifestNotFound,

    #[error("invalid TOML syntax")]
    #[diagnostic(code(skills_sync::manifest::invalid_toml))]
    InvalidToml {
        #[source_code]
        src: miette::NamedSource<String>,
        #[label("syntax error here")]
        span: miette::SourceSpan,
    },
}
```

### Pattern 4: Atomic File Writes via tempfile
**What:** Write to temp file in same directory, then rename
**When to use:** Writing metadata JSON files
**Example:**
```rust
use tempfile::NamedTempFile;
use std::io::Write;

fn write_metadata_atomic(path: &Path, meta: &SkillMeta) -> Result<()> {
    let dir = path.parent().unwrap();
    let mut tmp = NamedTempFile::new_in(dir)?;
    serde_json::to_writer_pretty(&mut tmp, meta)?;
    tmp.persist(path)?;
    Ok(())
}
```

### Pattern 5: Config Discovery Chain
**What:** Check CWD first, then XDG global, then `--config` override
**When to use:** Finding .skills.toml
**Example:**
```rust
fn discover_manifest(config_override: Option<&Path>) -> Result<PathBuf> {
    // 1. Explicit --config takes highest priority
    if let Some(path) = config_override {
        if path.exists() { return Ok(path.to_owned()); }
        return Err(ManifestNotFound { path: path.to_owned() });
    }
    // 2. CWD local manifest
    let local = Path::new(".skills.toml");
    if local.exists() { return Ok(local.to_owned()); }
    // 3. Global XDG config
    if let Some(config_dir) = dirs::config_dir() {
        let global = config_dir.join("skills-sync/skills.toml");
        if global.exists() { return Ok(global); }
    }
    Err(ManifestNotFound { searched: vec![...] })
}
```

### Anti-Patterns to Avoid
- **Panic on bad input:** Use `Result` returns and miette diagnostics, never `unwrap()` on user input
- **Hardcoded paths:** Use `dirs` crate for platform-appropriate paths
- **Silent data loss:** Always use atomic writes for metadata; never write directly to the target file
- **Stringly-typed errors:** Define error enum, don't use `anyhow!("something went wrong")`

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML parsing | Custom parser | `toml` crate | Edge cases in TOML spec (multiline strings, inline tables, etc.) |
| CLI arg parsing | Manual arg iteration | `clap` derive | Help text, validation, error messages all handled |
| Error display | Custom error formatting | `miette` | Source snippets, color, help text built-in |
| Atomic writes | Manual temp+rename | `tempfile::NamedTempFile::persist()` | Handles cleanup on failure, cross-platform |
| Config dirs | `$HOME/.config` hardcode | `dirs::config_dir()` | Respects XDG_CONFIG_HOME, works on macOS/Windows |
| Regex validation | Char-by-char checks | `regex` crate | Correct Unicode handling, compiled once |
| ISO 8601 timestamps | `format!()` with manual formatting | `chrono::Utc::now().to_rfc3339()` | Handles timezone, leap seconds, formatting |

**Key insight:** Every "simple" problem above has edge cases that bite after v1. The Rust ecosystem has battle-tested solutions for all of them.

## Common Pitfalls

### Pitfall 1: TOML Deserialization Strictness
**What goes wrong:** `serde` by default ignores unknown fields, which conflicts with the "warn on unknown keys" requirement.
**Why it happens:** Default `#[serde(deny_unknown_fields)]` rejects; default behavior silently ignores.
**How to avoid:** Use `toml::Value` first to detect unknown keys, then deserialize into typed struct. Or use `toml::Table` and manually check keys.
**Warning signs:** Tests pass with valid TOML but unknown keys slip through silently.

### Pitfall 2: Path Handling on Different Platforms
**What goes wrong:** Hardcoded `/` separators fail on Windows; `~` expansion not automatic.
**Why it happens:** `std::path::Path` doesn't expand `~`; `dirs` crate handles this.
**How to avoid:** Always use `PathBuf::join()` and `dirs::config_dir()`. Never string-concatenate paths.
**Warning signs:** Tests pass on macOS/Linux but fail on Windows CI.

### Pitfall 3: Error Context Loss
**What goes wrong:** Errors say "file not found" without saying which file.
**Why it happens:** Using `?` without adding context via `.map_err()` or miette labels.
**How to avoid:** Every I/O error should include the path that failed. Use miette's `#[diagnostic]` attributes.
**Warning signs:** Error messages in tests are generic rather than actionable.

### Pitfall 4: TOML Error Spans
**What goes wrong:** TOML parse errors don't include line/column info in diagnostic output.
**Why it happens:** The `toml` crate's error type includes span info, but you must extract and convert it to miette `SourceSpan`.
**How to avoid:** Catch `toml::de::Error`, extract `.span()`, convert to `miette::SourceSpan`, include original source as `NamedSource`.
**Warning signs:** Parse errors show "invalid TOML" without pointing to the problematic line.

### Pitfall 5: NO_COLOR Respect
**What goes wrong:** Tool outputs ANSI codes even when piped or when `NO_COLOR` is set.
**Why it happens:** Using `owo-colors` without checking terminal capability first.
**How to avoid:** Use `supports-color` crate to check, or use `owo-colors`'s `if_supports_color()` method. Check `NO_COLOR` env var.
**Warning signs:** CI logs full of ANSI escape sequences.

## Code Examples

### Manifest Struct with Serde
```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub skills: Skills,
}

#[derive(Debug, Deserialize)]
pub struct Skills {
    pub repos: Vec<String>,
}

pub fn parse_manifest(content: &str) -> Result<Manifest, toml::de::Error> {
    toml::from_str(content)
}
```

### Repo Pattern Validation
```rust
use regex::Regex;
use std::sync::LazyLock;

static REPO_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+$").unwrap()
});

pub fn validate_repo(repo: &str) -> bool {
    REPO_PATTERN.is_match(repo.trim())
}
```

### Metadata Struct
```rust
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SkillMeta {
    pub schema_version: u32,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirectory: Option<String>,
    pub sha: String,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SkillMeta {
    pub fn new(source: String, sha: String) -> Self {
        let now = Utc::now();
        Self {
            schema_version: 1,
            source,
            subdirectory: None,
            sha,
            installed_at: now,
            updated_at: now,
        }
    }
}
```

### Init Command
```rust
pub fn run_init(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(SkillsSyncError::ManifestAlreadyExists {
            path: path.to_owned(),
        });
    }

    let template = r#"# skills-sync manifest
# ...tutorial content...

[skills]
repos = [
    # "anthropics/claude-skills",    # single-skill repo
    # "owner/monorepo",              # monorepo with multiple skills
]
"#;

    std::fs::write(path, template)?;
    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `lazy_static!` for regex | `std::sync::LazyLock` (stable) | Rust 1.80 (2024) | No external crate needed for lazy statics |
| `structopt` for CLI | `clap` derive (v4) | clap 3.0 (2022) | structopt merged into clap |
| `anyhow` for all errors | `thiserror` + `miette` for CLIs | miette 5+ (2023) | Better diagnostic output |
| Manual NO_COLOR check | `supports-color` crate | 2023 | Standard detection including CI |

**Deprecated/outdated:**
- `structopt`: Merged into clap v3+, use `clap` derive directly
- `lazy_static!`: Use `std::sync::LazyLock` (stable since Rust 1.80)
- `failure` crate: Abandoned, use `thiserror`

## Open Questions

1. **Exact toml crate version compatibility**
   - What we know: toml 0.8 and 0.9 are both recent; 0.9 is very new
   - What's unclear: Whether 0.9 has breaking changes affecting our usage
   - Recommendation: Use 0.8 for stability; 0.9 if available and compatible

2. **miette + owo-colors integration**
   - What we know: miette has its own color support via `fancy` feature
   - What's unclear: Whether additional owo-colors is needed or redundant
   - Recommendation: Use miette's built-in coloring for errors; owo-colors only for non-error output (warnings, success messages)

## Sources

### Primary (HIGH confidence)
- crates.io registry — version numbers and download counts for all crates
- docs.rs — API documentation for toml, clap, serde, miette, thiserror, tempfile, dirs

### Secondary (MEDIUM confidence)
- WebSearch results for current crate versions and ecosystem patterns
- Rust CLI Recommendations (rust-cli-recommendations.sunshowers.io) — color management guidance

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates are well-established with millions of downloads
- Architecture: HIGH — patterns follow standard Rust CLI conventions
- Pitfalls: HIGH — based on well-documented community experience

**Research date:** 2026-02-28
**Valid until:** 2026-03-28 (stable ecosystem, 30-day window)
