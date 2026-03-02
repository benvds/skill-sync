# Phase 4: CLI + UX - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Full CLI surface with all flags working (--dry-run, --force, --verbose, --version, --help, --global/--local mutual exclusivity), color-coded output, progress feedback, and binary distribution via GitHub Releases. Existing sync engine and resolver are stable — this phase layers UX and distribution on top.

</domain>

<decisions>
## Implementation Decisions

### Output styling
- Minimal with color, like cargo/rustup — colored status words (green "installed", yellow "updated", red "failed"), no fancy Unicode glyphs or box drawing
- Auto-detect TTY for color output: colors on when terminal, off when piped/redirected. No --no-color flag needed
- One-line sync summary at the end, colored: "sync complete: 3 installed, 1 updated, 0 removed"

### Dry-run presentation
- Flat list with action prefixes: `+ owner/repo (install)`, `~ owner/repo (update)`, `- owner/repo (remove)`
- Still fetches from GitHub API to determine remote state — not offline. Skips git clone/remove operations only
- Brief by default — just action + skill name. SHA details shown only when combined with --verbose
- Exit code 2 when changes are pending (exit 0 = everything up to date). Useful for CI drift detection

### Progress feedback
- Line-by-line status as actions happen: print each action as it starts and completes. Simple, scriptable, no terminal tricks (no spinners/progress bars)
- --verbose shows per-skill SHA comparisons (local vs remote) and git command stderr output
- --force auto-accepts all confirmation prompts (including orphan removal). Clean CI behavior
- No timing info in output — users can use `time skills-sync` if needed

### Distribution
- GitHub Releases only — no Homebrew tap for now
- Target platforms: macOS ARM (apple-silicon) and Linux x86_64
- GitHub Actions CI triggered on git tag (e.g., v0.1.0) to build and upload release binaries
- Include an install script (`curl -fsSL ... | sh`) that detects OS/arch and downloads the right binary from Releases

### Claude's Discretion
- Color library choice (owo-colors, colored, or similar — pick based on dependency weight)
- Exact color assignments beyond the green/yellow/red status words
- Install script implementation details (install location, PATH handling)
- GitHub Actions workflow structure

</decisions>

<specifics>
## Specific Ideas

- Output should feel like cargo or rustup — familiar to Rust/CLI users
- --dry-run exit code 2 for pending changes enables CI drift detection (e.g., `skills-sync --dry-run || echo "skills out of sync"`)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/cli.rs`: Clap derive struct — straightforward to add new flags (--dry-run, --force, --version, --local)
- `src/error.rs`: miette + thiserror diagnostics — error presentation is already polished with codes and help text
- `src/sync.rs`: `execute_sync_plan()` handles the full sync lifecycle — dry-run needs to short-circuit before this
- `dialoguer`: Already a dependency for interactive prompts — --force needs to bypass `prompt_orphan_removal()`

### Established Patterns
- All output uses `println!` (stdout) for results and `eprintln!` (stderr) for warnings/diagnostics
- Verbose output gates on `cli.verbose` flag checked inline in main.rs
- Error flow: `run()` returns `Result`, main prints error and exits with code 1
- Clap derive pattern with `#[arg(long)]` attributes

### Integration Points
- `main.rs:run()` — needs --dry-run check after resolver produces actions but before execute_sync_plan
- `main.rs:run()` — needs --force passed through to sync engine to skip prompts
- `cli.rs:Cli` struct — add --dry-run, --force, --version, --local flags + mutual exclusivity group
- `sync.rs:prompt_orphan_removal()` — needs force parameter to auto-confirm
- `Cargo.toml` — add color library dependency

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 04-cli-ux*
*Context gathered: 2026-03-02*
