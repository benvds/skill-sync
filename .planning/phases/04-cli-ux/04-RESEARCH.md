# Phase 4: CLI + UX - Research

**Researched:** 2026-03-02
**Domain:** Rust CLI flag implementation, terminal color output, and GitHub Actions binary distribution
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Output styling**
- Minimal with color, like cargo/rustup — colored status words (green "installed", yellow "updated", red "failed"), no fancy Unicode glyphs or box drawing
- Auto-detect TTY for color output: colors on when terminal, off when piped/redirected. No --no-color flag needed
- One-line sync summary at the end, colored: "sync complete: 3 installed, 1 updated, 0 removed"

**Dry-run presentation**
- Flat list with action prefixes: `+ owner/repo (install)`, `~ owner/repo (update)`, `- owner/repo (remove)`
- Still fetches from GitHub API to determine remote state — not offline. Skips git clone/remove operations only
- Brief by default — just action + skill name. SHA details shown only when combined with --verbose
- Exit code 2 when changes are pending (exit 0 = everything up to date). Useful for CI drift detection

**Progress feedback**
- Line-by-line status as actions happen: print each action as it starts and completes. Simple, scriptable, no terminal tricks (no spinners/progress bars)
- --verbose shows per-skill SHA comparisons (local vs remote) and git command stderr output
- --force auto-accepts all confirmation prompts (including orphan removal). Clean CI behavior
- No timing info in output — users can use `time skills-sync` if needed

**Distribution**
- GitHub Releases only — no Homebrew tap for now
- Target platforms: macOS ARM (apple-silicon) and Linux x86_64
- GitHub Actions CI triggered on git tag (e.g., v0.1.0) to build and upload release binaries
- Include an install script (`curl -fsSL ... | sh`) that detects OS/arch and downloads the right binary from Releases

### Claude's Discretion
- Color library choice (owo-colors, colored, or similar — pick based on dependency weight)
- Exact color assignments beyond the green/yellow/red status words
- Install script implementation details (install location, PATH handling)
- GitHub Actions workflow structure

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-01 | `--dry-run` shows planned changes (install/update/remove) without executing | Dry-run short-circuits before `execute_sync_plan()`; insert check after `all_actions` built in `run()` |
| CLI-02 | `--force` skips confirmation prompts for non-interactive/CI usage | Pass `force: bool` through to `prompt_orphan_removal()`; bypass dialoguer call when true |
| CLI-03 | `--verbose` shows detailed per-skill output including SHA comparisons | Already partially implemented via `cli.verbose` gates in `main.rs`; extend to show old_sha/new_sha in Update actions |
| CLI-04 | `--help` displays usage information with all flags and options | Provided automatically by clap; ensure all fields have doc comments; use `#[command(version, about)]` |
| CLI-05 | `--version` displays current version | `#[command(version)]` in clap reads from `Cargo.toml` version field automatically |
| CLI-06 | `--global` and `--local` are mutually exclusive; conflicting usage produces clear error | Use `#[arg(long, conflicts_with = "global")]` on `--local` field; clap produces error before any API calls |
</phase_requirements>

## Summary

Phase 4 adds the remaining CLI surface (--dry-run, --force, --version, --local flags and mutual exclusivity enforcement), applies owo-colors for TTY-aware colored output, and creates a GitHub Actions release workflow for macOS ARM and Linux x86_64 binaries.

The existing codebase already has a clap derive struct in `src/cli.rs` with `--verbose` and `--global`. All flag additions follow the same `#[arg(long)]` derive pattern. Mutual exclusivity between `--global` and `--local` is handled by clap's built-in `conflicts_with` attribute — no custom validation code needed. The error fires before any network calls because clap parses and validates before `run()` is entered.

For coloring, owo-colors (v4.x) with the `supports-colors` feature is the recommended choice per the Rust CLI community guide. It handles TTY detection, `NO_COLOR`, `FORCE_COLOR`, and CI environments automatically via `if_supports_color(Stream, closure)`. For distribution, the `houseabsolute/actions-rust-cross` GitHub Action is the modern standard: it handles native Apple Silicon runners (macos-latest = ARM since 2024) and uses `cross` only when needed for Linux cross-compilation.

**Primary recommendation:** Use owo-colors with the `supports-colors` feature; add `conflicts_with` to clap fields; distribute via `houseabsolute/actions-rust-cross` + `softprops/action-gh-release`.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4 (already in Cargo.toml) | CLI parsing, --help, --version, mutual exclusivity | Already used; derive macro handles all cases |
| owo-colors | 4.x | TTY-aware terminal coloring | Zero-allocation, no_std-compatible; only Rust CLI lib that passes all criteria per rain.sunshowers.io guide; recommended over `colored` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dialoguer | 0.12 (already in Cargo.toml) | Interactive prompts | Already used; `--force` bypasses it |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| owo-colors | colored | `colored` uses global mutable state; owo-colors is stream-aware and has no allocations |
| owo-colors | termcolor | Lower-level; more boilerplate; better for complex multi-stream apps |
| houseabsolute/actions-rust-cross | actions-rs/cargo + cross | actions-rs is deprecated; houseabsolute is the maintained successor |
| softprops/action-gh-release | gh release create | softprops handles asset matrix aggregation cleanly; gh CLI works too for simple cases |

**Installation:**
```bash
# Add to Cargo.toml [dependencies]
owo-colors = { version = "4", features = ["supports-colors"] }
```

## Architecture Patterns

### Recommended Project Structure

No structural changes needed. Changes are in-place edits to:
```
src/
├── cli.rs       # Add --dry-run, --force, --version, --local flags + conflicts_with
├── main.rs      # Add dry-run short-circuit, force passthrough, colored output
├── sync.rs      # Modify prompt_orphan_removal() to accept force: bool
└── error.rs     # No changes needed (miette already handles errors well)
.github/
└── workflows/
    └── release.yml  # New: GitHub Actions release workflow
install.sh           # New: curl-based installer script
```

### Pattern 1: Clap `conflicts_with` for Mutual Exclusivity

**What:** Declare `--local` conflicts with `--global` directly on the struct field. Clap validates before `run()` is called — no network calls happen.
**When to use:** Any pair of flags that are logically mutually exclusive.

```rust
// Source: https://docs.rs/clap/latest/clap/struct.Arg.html#method.conflicts_with
// and https://users.rust-lang.org/t/clap-mutually-exclusive-options/97476

#[derive(Parser, Debug)]
#[command(name = "skills-sync", about = "Sync Claude skills from manifest", version)]
pub struct Cli {
    /// Generate a sample .skills.toml
    #[arg(long)]
    pub init: bool,

    /// Path to config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Show verbose output including SHA comparisons and git command output
    #[arg(long)]
    pub verbose: bool,

    /// Sync to global target (~/.claude/skills/) instead of local
    #[arg(long)]
    pub global: bool,

    /// Sync to local target (.claude/skills/) — default behavior, mutually exclusive with --global
    #[arg(long, conflicts_with = "global")]
    pub local: bool,

    /// Show planned changes without executing any installs, updates, or removals
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts; auto-accept all changes (for CI)
    #[arg(long)]
    pub force: bool,
}
```

The `conflicts_with = "global"` attribute uses the field name (not the flag name). Clap automatically generates a clear error: `error: the argument '--local' cannot be used with '--global'`.

### Pattern 2: Clap `#[command(version)]` for --version

**What:** Adding `version` to the `#[command(...)]` macro automatically reads the version from `Cargo.toml` and wires `--version`.
**When to use:** Always — never hard-code version strings.

```rust
// Source: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
#[command(name = "skills-sync", about = "Sync Claude skills from manifest", version)]
pub struct Cli { ... }
```

With `version` in the attribute, `skills-sync --version` prints `skills-sync 0.1.0` (read from Cargo.toml `version` field). `--help` also shows the version automatically.

### Pattern 3: owo-colors Stream-Aware Coloring

**What:** Use `if_supports_color(Stream, closure)` to apply color only when the stream is a TTY. Respects `NO_COLOR` and `FORCE_COLOR` environment variables automatically with the `supports-colors` feature enabled.
**When to use:** Every colored output line. Pass the correct stream (Stdout vs Stderr) matching where the output goes.

```rust
// Source: https://docs.rs/owo-colors/latest/owo_colors/
// and https://github.com/owo-colors/owo-colors/blob/main/README.md
use owo_colors::{OwoColorize, Stream};

// For stdout results (installed/updated/removed/skipped lines)
println!("{} {}", "installed:".if_supports_color(Stream::Stdout, |t| t.green()), dir_name);
println!("{} {}", "updated:".if_supports_color(Stream::Stdout, |t| t.yellow()), dir_name);
println!("{} {}", "failed:".if_supports_color(Stream::Stdout, |t| t.red()), dir_name);

// For the final summary (stdout)
println!(
    "{}",
    format!("sync complete: {} installed, {} updated, {} removed", installs, updates, removes)
        .if_supports_color(Stream::Stdout, |t| t.green())
);

// For warnings (stderr)
eprintln!("{} {}", "warning:".if_supports_color(Stream::Stderr, |t| t.yellow()), msg);
```

The `supports-colors` feature must be enabled in Cargo.toml for `if_supports_color` to be available:
```toml
owo-colors = { version = "4", features = ["supports-colors"] }
```

### Pattern 4: Dry-Run Short-Circuit

**What:** After the resolver produces `all_actions` but before `execute_sync_plan()` is called, check `cli.dry_run` and print the plan then exit.
**When to use:** The integration point is the section in `main.rs:run()` between "Report sync plan summary" and "Execute sync plan".

```rust
// In main.rs run() — insert after all_actions built, before execute_sync_plan call:
if cli.dry_run {
    // Print each planned action
    for action in &all_actions {
        match action {
            resolver::SyncAction::Install { repo, subdirectory, .. } => {
                let name = format_skill_name(repo, subdirectory);
                println!("+ {} (install)", name);
            }
            resolver::SyncAction::Update { repo, subdirectory, old_sha, new_sha, .. } => {
                let name = format_skill_name(repo, subdirectory);
                if cli.verbose {
                    println!("~ {} (update) {} -> {}", name, &old_sha[..8], &new_sha[..8]);
                } else {
                    println!("~ {} (update)", name);
                }
            }
            resolver::SyncAction::Remove { repo, subdirectory, .. } => {
                let name = format_skill_name(repo, subdirectory);
                println!("- {} (remove)", name);
            }
            resolver::SyncAction::Skip { .. } => {} // Omit from dry-run output
        }
    }

    let pending = all_actions.iter().filter(|a| !matches!(a, resolver::SyncAction::Skip { .. })).count();
    if pending > 0 {
        std::process::exit(2); // Exit code 2 = changes pending
    }
    return Ok(()); // Exit code 0 = everything up to date
}
```

### Pattern 5: Force Flag Threaded to Prompt

**What:** Pass `cli.force` through `execute_sync_plan()` to `prompt_orphan_removal()`. When force is true, skip the dialoguer call and return `true` directly.
**When to use:** The existing `prompt_orphan_removal()` function in `sync.rs` takes only `orphans: &[OrphanSkill]`; it needs a `force: bool` parameter.

```rust
// Modified prompt_orphan_removal signature in sync.rs:
fn prompt_orphan_removal(orphans: &[OrphanSkill], force: bool) -> bool {
    if force {
        // In force mode, auto-confirm removal without any output
        return true;
    }
    // ... existing dialoguer code unchanged ...
}

// execute_sync_plan gains a force parameter:
pub fn execute_sync_plan(
    actions: &[crate::resolver::SyncAction],
    target_dir: &Path,
    token: Option<&str>,
    branch_map: &HashMap<String, String>,
    manifest_repos: &HashSet<String>,
    lock_path: &Path,
    force: bool,  // NEW
) -> crate::error::Result<()> {
    // ...
    let confirmed = prompt_orphan_removal(&all_orphans, force);
    // ...
}
```

### Pattern 6: GitHub Actions Release Workflow

**What:** Matrix build triggered by version tags. macOS ARM uses native `macos-latest` runner (Apple Silicon since 2024). Linux uses `ubuntu-latest` with musl target for portable static binaries.
**When to use:** Triggered by `git tag v0.1.0 && git push --tags`.

```yaml
# Source: https://github.com/houseabsolute/actions-rust-cross
# and https://dzfrias.dev/blog/deploy-rust-cross-platform-github-actions/
name: Release

on:
  push:
    tags:
      - "v[0-9]+.[0-9]+.[0-9]+"

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.platform.name }}
    runs-on: ${{ matrix.platform.runs-on }}
    strategy:
      matrix:
        platform:
          - name: macOS ARM64
            runs-on: macos-latest
            target: aarch64-apple-darwin
          - name: Linux x86_64
            runs-on: ubuntu-latest
            target: x86_64-unknown-linux-musl
    steps:
      - uses: actions/checkout@v4

      - uses: houseabsolute/actions-rust-cross@v1
        with:
          command: build
          target: ${{ matrix.platform.target }}
          args: "--locked --release"
          strip: true

      - name: Package binary
        shell: bash
        run: |
          VERSION="${GITHUB_REF#refs/tags/}"
          TARGET="${{ matrix.platform.target }}"
          BINARY="skills-sync"
          ARCHIVE="${BINARY}-${VERSION}-${TARGET}.tar.gz"
          cp "target/${TARGET}/release/${BINARY}" "${BINARY}"
          tar -czf "${ARCHIVE}" "${BINARY}"
          echo "ARCHIVE=${ARCHIVE}" >> $GITHUB_ENV

      - uses: softprops/action-gh-release@v2
        with:
          files: ${{ env.ARCHIVE }}
```

### Pattern 7: Install Script Structure

**What:** A POSIX shell script (`install.sh`) that detects OS and architecture via `uname`, selects the right binary from GitHub Releases, downloads it, installs to `~/.local/bin` (or `/usr/local/bin` if available), and prints a PATH hint.
**When to use:** `curl -fsSL https://github.com/USER/skills-sync/releases/latest/download/install.sh | sh`

```sh
#!/bin/sh
set -e

REPO="OWNER/skills-sync"
BIN_NAME="skills-sync"
INSTALL_DIR="${HOME}/.local/bin"

OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}-${ARCH}" in
  Darwin-arm64)  TARGET="aarch64-apple-darwin" ;;
  Linux-x86_64)  TARGET="x86_64-unknown-linux-musl" ;;
  *)
    echo "Unsupported platform: ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | cut -d'"' -f4)

URL="https://github.com/${REPO}/releases/download/${VERSION}/${BIN_NAME}-${VERSION}-${TARGET}.tar.gz"

mkdir -p "${INSTALL_DIR}"
curl -fsSL "${URL}" | tar -xz -C "${INSTALL_DIR}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo "Installed ${BIN_NAME} ${VERSION} to ${INSTALL_DIR}/${BIN_NAME}"
echo "Make sure ${INSTALL_DIR} is in your PATH."
```

### Anti-Patterns to Avoid

- **Setting color globally:** Avoid `owo_colors::set_override(true)` — it affects all output including piped; use `if_supports_color(Stream, ...)` per-call instead.
- **Checking TTY manually:** Do not call `atty::is_tty()` or `std::io::stdout().is_terminal()` and then conditionally apply color — let owo-colors handle this via `if_supports_color`.
- **Custom mutual exclusivity validation:** Do not validate `--global` and `--local` in `run()` with an `if cli.global && cli.local { return Err(...) }` check — clap's `conflicts_with` handles this before the function is called, giving a better error format.
- **Hard-coding version string:** Never `const VERSION: &str = "0.1.0"` — use clap's `#[command(version)]` which reads from `Cargo.toml`.
- **Using linux gnu musl-builder for musl:** Use `x86_64-unknown-linux-musl` with the `cross` tool via `houseabsolute/actions-rust-cross` — do not attempt to install musl-tools manually.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TTY detection + NO_COLOR support | Custom `is_tty()` + env var checking | owo-colors `if_supports_color` with `supports-colors` feature | Handles CI detection, FORCE_COLOR, NO_COLOR, terminal type detection — at least 4 edge cases |
| --version flag | `const VERSION: &str = "..."` | `#[command(version)]` in clap | Eliminates version drift between code and Cargo.toml |
| Mutual exclusivity error | Custom validation in `run()` | `conflicts_with = "field_name"` in clap | Clap formats the error consistently with help text; fires before any code runs |
| Cross-compilation setup | Manual target install + cargo flags | `houseabsolute/actions-rust-cross` | Handles cross/cargo selection, caching, stripping automatically |

**Key insight:** The clap and owo-colors libraries handle all CLI validation and output color edge cases that are deceptively complex to get right (NO_COLOR, CI detection, FORCE_COLOR, error formatting).

## Common Pitfalls

### Pitfall 1: `--local` Field vs Flag Name

**What goes wrong:** Adding `pub local: bool` to the Cli struct but forgetting that clap's derive uses the field name (not the flag name) in `conflicts_with`. If the field is named `local_scope`, `conflicts_with = "local"` won't compile.
**Why it happens:** `conflicts_with` takes the argument ID, which is the field name by default in derive mode.
**How to avoid:** Keep the field name exactly matching the string passed to `conflicts_with`. With `pub local: bool`, use `conflicts_with = "global"` on `local` (and optionally `conflicts_with = "local"` on `global` for symmetric checking).
**Warning signs:** Compile error "no such argument 'X'" at build time.

### Pitfall 2: Dry-Run Orphan Count

**What goes wrong:** The dry-run output omits orphan removal actions from the count because `all_actions` only contains resolver `Remove` actions for skills within a repo — global orphans (repos removed entirely from manifest) are discovered later inside `execute_sync_plan()`.
**Why it happens:** `scan_orphans()` is called inside `execute_sync_plan()`, not in `run()`. The dry-run short-circuit fires before that.
**How to avoid:** Either move `scan_orphans()` call into `run()` before the dry-run check, or accept that dry-run only shows resolver-level Remove actions (not global orphans). Document this limitation. Moving it to `run()` is cleaner.
**Warning signs:** `--dry-run` shows `0 remove` but actual sync removes a skill.

### Pitfall 3: Exit Code 2 vs process::exit vs Ok(())

**What goes wrong:** Using `std::process::exit(2)` in dry-run mode bypasses Rust's stack unwinding and drop order, which can leave locks uncleaned. Alternatively, returning `Ok(())` always gives exit code 0.
**Why it happens:** The existing `run()` returns `Result<()>` — non-zero exit is only triggered by `Err`. There's no built-in mechanism for "success but non-zero".
**How to avoid:** `std::process::exit(2)` is correct here — there are no locks or resources that need cleanup at this point in the flow (dry-run doesn't touch disk or network beyond the API calls already done). Just call it before `return Ok(())`.
**Warning signs:** Exit code always 0 even when changes are pending.

### Pitfall 4: musl vs gnu for Linux

**What goes wrong:** Building Linux binaries with `x86_64-unknown-linux-gnu` instead of `x86_64-unknown-linux-musl` produces binaries that fail on different glibc versions.
**Why it happens:** gnu is the default target, musl produces static binaries that run anywhere.
**How to avoid:** Always use `x86_64-unknown-linux-musl` for released CLI binaries. `houseabsolute/actions-rust-cross` handles the musl toolchain automatically.
**Warning signs:** Binary fails with "version 'GLIBC_2.XX' not found" on older Linux distributions.

### Pitfall 5: macos-latest ARM Runner Assumption

**What goes wrong:** Assuming `macos-latest` is still x86_64 and adding `aarch64-apple-darwin` as a cross-compilation target when it's now native.
**Why it happens:** Prior to 2024, `macos-latest` was Intel. Since January 2024, GitHub Actions provides Apple Silicon runners for public repos, and `macos-latest` now maps to ARM.
**How to avoid:** Use `runs-on: macos-latest` with `target: aarch64-apple-darwin` — this compiles natively without cross. No cross-compilation overhead.
**Warning signs:** Build takes 3x longer than expected (cross-compilation from Intel).

### Pitfall 6: Color on stderr vs stdout Stream Mismatch

**What goes wrong:** Using `Stream::Stdout` when writing to `eprintln!` (stderr) — TTY detection fires on the wrong stream.
**Why it happens:** stdout and stderr can have different TTY status (e.g., stdout piped, stderr to terminal).
**How to avoid:** Match the stream enum to the actual output: `println!` → `Stream::Stdout`, `eprintln!` → `Stream::Stderr`.
**Warning signs:** Color escapes appear in piped output.

## Code Examples

Verified patterns from official sources:

### Adding --dry-run, --force, --local, --version to Cli struct

```rust
// src/cli.rs
// Source: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "skills-sync", about = "Sync Claude skills from manifest", version)]
pub struct Cli {
    /// Generate a sample .skills.toml
    #[arg(long)]
    pub init: bool,

    /// Path to config file
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Show verbose output including SHA comparisons and git command output
    #[arg(long)]
    pub verbose: bool,

    /// Sync to global target (~/.claude/skills/) instead of local
    #[arg(long)]
    pub global: bool,

    /// Sync to local target (.claude/skills/) — mutually exclusive with --global
    #[arg(long, conflicts_with = "global")]
    pub local: bool,

    /// Show planned changes without executing (exit 2 if changes pending)
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts for CI usage
    #[arg(long)]
    pub force: bool,
}
```

### Colored output using owo-colors

```rust
// In sync.rs or main.rs
// Source: https://docs.rs/owo-colors/latest/owo_colors/
// Source: https://github.com/owo-colors/owo-colors/blob/main/README.md
use owo_colors::{OwoColorize, Stream};

// Success output (stdout)
println!(
    "{} {}",
    "installed:".if_supports_color(Stream::Stdout, |t| t.green()),
    dir_name
);

// Warning output (stderr)
eprintln!(
    "{} {}",
    "warning:".if_supports_color(Stream::Stderr, |t| t.yellow()),
    message
);

// Summary line (stdout)
let summary = format!("sync complete: {} installed, {} updated, {} removed", i, u, r);
println!("{}", summary.if_supports_color(Stream::Stdout, |t| t.green()));
```

### Dry-run action rendering

```rust
// In main.rs:run() — after all_actions collected, before execute_sync_plan
// Source: pattern derived from CONTEXT.md decisions
use owo_colors::{OwoColorize, Stream};

if cli.dry_run {
    for action in &all_actions {
        match action {
            resolver::SyncAction::Install { repo, subdirectory, .. } => {
                let name = resolver::skill_key(repo, subdirectory);
                println!(
                    "{} {} (install)",
                    "+".if_supports_color(Stream::Stdout, |t| t.green()),
                    name
                );
            }
            resolver::SyncAction::Update { repo, subdirectory, old_sha, new_sha, .. } => {
                let name = resolver::skill_key(repo, subdirectory);
                if cli.verbose {
                    println!(
                        "{} {} (update: {}..{})",
                        "~".if_supports_color(Stream::Stdout, |t| t.yellow()),
                        name,
                        &old_sha[..8],
                        &new_sha[..8]
                    );
                } else {
                    println!(
                        "{} {} (update)",
                        "~".if_supports_color(Stream::Stdout, |t| t.yellow()),
                        name
                    );
                }
            }
            resolver::SyncAction::Remove { repo, subdirectory, .. } => {
                let name = resolver::skill_key(repo, subdirectory);
                println!(
                    "{} {} (remove)",
                    "-".if_supports_color(Stream::Stdout, |t| t.red()),
                    name
                );
            }
            resolver::SyncAction::Skip { .. } => {}
        }
    }

    let pending = all_actions.iter()
        .filter(|a| !matches!(a, resolver::SyncAction::Skip { .. }))
        .count();
    if pending > 0 {
        std::process::exit(2);
    }
    return Ok(());
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `colored` crate | `owo-colors` with `supports-colors` feature | ~2022-2023 | owo-colors is zero-allocation, stream-aware; now the ecosystem recommendation |
| `actions-rs/cargo` | `houseabsolute/actions-rust-cross` | 2023-2024 | actions-rs deprecated; houseabsolute is the maintained successor |
| Intel macOS GitHub runners | Apple Silicon `macos-latest` | January 2024 | `aarch64-apple-darwin` is now native — no cross-compilation needed |
| Cross-compile musl manually | `cross` tool via actions-rust-cross | 2022+ | `cross` handles musl toolchain in Docker automatically |

**Deprecated/outdated:**
- `actions-rs/cargo`: Archived/deprecated on GitHub; do not use.
- `colored` crate: Still works but uses global mutable state; owo-colors is preferred for new projects.
- Cross-compiling macOS ARM on Intel runners: GitHub now provides native Apple Silicon runners; no need for Intel cross-compilation setup.

## Open Questions

1. **`--local` flag necessity**
   - What we know: `--local` is listed in CLI-06 requirements as part of mutual exclusivity; the default behavior is already local (`.claude/skills/`)
   - What's unclear: Does `--local` need to explicitly override a config file that might set global, or is it purely for symmetry with `--global`?
   - Recommendation: Implement as a no-op flag that simply enforces the default (local) and prevents `--global` from being used. No additional behavior needed beyond the mutual exclusivity enforcement.

2. **Global orphan scan in dry-run mode**
   - What we know: `scan_orphans()` is currently called inside `execute_sync_plan()`, which dry-run short-circuits before calling
   - What's unclear: Should dry-run show global orphans (repos removed from manifest entirely)?
   - Recommendation: Move `scan_orphans()` call into `run()` before the dry-run check. Add global orphan actions to `all_actions` as `Remove` entries. This makes dry-run output complete and accurate.

3. **`--help` output format**
   - What we know: Clap generates `--help` automatically from doc comments on fields
   - What's unclear: Whether the current `about` text in `#[command(...)]` adequately describes the tool for v1
   - Recommendation: Update the `about` string and ensure all new flag fields have clear doc comments — this is the only work needed.

## Sources

### Primary (HIGH confidence)
- https://docs.rs/clap/latest/clap/builder/struct.ArgGroup.html — conflicts_with API
- https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html — derive macro version, conflicts_with, `#[command(version)]`
- https://docs.rs/owo-colors/4.2.3/owo_colors/ — OwoColorize trait, if_supports_color, Stream enum
- https://github.com/owo-colors/owo-colors/blob/main/README.md — supports-colors feature, usage patterns
- https://github.com/houseabsolute/actions-rust-cross — action inputs, matrix example
- https://rust-cli-recommendations.sunshowers.io/managing-colors-in-rust.html — owo-colors recommendation

### Secondary (MEDIUM confidence)
- https://dzfrias.dev/blog/deploy-rust-cross-platform-github-actions/ — full workflow YAML verified against GitHub Actions syntax
- https://ahmedjama.com/blog/2025/12/cross-platform-rust-pipeline-github-actions/ — trigger pattern, softprops/action-gh-release v2 usage
- https://users.rust-lang.org/t/clap-mutually-exclusive-options/97476 — `conflicts_with` derive macro attribute pattern

### Tertiary (LOW confidence)
- GitHub Actions Apple Silicon runner availability since January 2024 — search result claim; verify against official GitHub docs before finalizing workflow

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — owo-colors and clap verified against official docs.rs; houseabsolute/actions-rust-cross verified on GitHub
- Architecture: HIGH — clap derive patterns verified against official docs; owo-colors patterns verified against official docs and README
- Pitfalls: MEDIUM — musl vs gnu and exit code behavior from experience + community docs; Apple Silicon runner timeline from search result (verify)
- Distribution workflow: MEDIUM — YAML patterns from blog posts verified against action repos; softprops/action-gh-release v2 confirmed on GitHub Marketplace

**Research date:** 2026-03-02
**Valid until:** 2026-04-02 (stable libraries, 30-day estimate)
