---
phase: 04-cli-ux
plan: 02
subsystem: cli-output
tags: [colors, owo-colors, verbose, release, install-script, github-actions]
dependency_graph:
  requires: ["04-01"]
  provides: ["colored-output", "verbose-sha", "release-workflow", "install-script"]
  affects: ["src/sync.rs", "src/main.rs", "Cargo.toml"]
tech_stack:
  added: ["owo-colors 4 (supports-colors feature)"]
  patterns: ["if_supports_color per-call TTY detection", "cargo/rustup color style"]
key_files:
  created:
    - ".github/workflows/release.yml"
    - "install.sh"
  modified:
    - "Cargo.toml"
    - "src/main.rs"
    - "src/sync.rs"
decisions:
  - "verbose SHA shown before operation (not after) so it precedes the result line"
  - "install_count/update_count/remove_count tracked separately for accurate summary"
  - "Summary line green on full success, red on any failure (replaces old 'succeeded/failed' format)"
  - "install.sh targets ~/.local/bin as default — standard XDG user bin location"
  - "install.sh uses POSIX sh (not bash) for broad compatibility"
metrics:
  duration: "~3 min"
  completed: "2026-03-02"
  tasks_completed: 3
  files_changed: 5
---

# Phase 4 Plan 2: Colored Output, Verbose SHA, and Release Infrastructure Summary

TTY-aware colored CLI output via owo-colors, verbose SHA comparisons per skill, GitHub Actions release workflow building macOS ARM + Linux x86_64 binaries, and a curl-based install script.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add owo-colors and colored output | f06c751 | Cargo.toml, src/main.rs, src/sync.rs |
| 2 | Enhance --verbose with SHA comparisons | d9514fb | src/sync.rs |
| 3 | Create release workflow and install script | b9ca90e | .github/workflows/release.yml, install.sh |

## What Was Built

### Task 1: Colored Output

Added `owo-colors = { version = "4", features = ["supports-colors"] }` to Cargo.toml. Applied `if_supports_color` per-call TTY detection throughout:

- **src/sync.rs**: Status words colored — `installed:` green, `updated:` yellow, `removed:` green, `failed:` red. Orphan warning prefix `warning:` yellow. Summary line colored green on success, red on failure. Counters split into `install_count`, `update_count`, `remove_count` for accurate "sync complete: N installed, N updated, N removed" format.
- **src/main.rs**: Dry-run prefixes colored — `+` green (install), `~` yellow (update), `-` red (remove). All `warning:` prefixes colored yellow on stderr. "everything up to date" message colored green. Correct `Stream::Stdout` vs `Stream::Stderr` matching throughout.

Colors auto-disable when output is piped (owo-colors `supports-colors` feature handles TTY detection automatically — no `--no-color` flag needed).

### Task 2: Verbose SHA Comparisons

Added `verbose: bool` parameter to `execute_sync_plan` signature. SHA details printed to stderr before each operation:

- **Install**: `  dir_name -> abcd1234` (first 8 chars of remote SHA)
- **Update**: `  dir_name old_sha12 -> new_sha56` (first 8 chars of both SHAs)
- **Skip**: `  skip: dir_name (sha_12345)` (first 8 chars of current SHA)

`cli.verbose` passed through from `main.rs` to `execute_sync_plan`.

### Task 3: Release Infrastructure

**`.github/workflows/release.yml`**: Matrix build triggered on `v*.*.*` tag push. Two targets: `aarch64-apple-darwin` (macOS ARM64, runs on macos-latest) and `x86_64-unknown-linux-musl` (Linux x86_64, runs on ubuntu-latest). Uses `houseabsolute/actions-rust-cross@v1` with `--locked --release` and strip enabled. Archives as `skills-sync-{version}-{target}.tar.gz` uploaded via `softprops/action-gh-release@v2`.

**`install.sh`**: POSIX sh (`set -e`) installer. Detects `Darwin-arm64` and `Linux-x86_64` via `uname -s` / `uname -m`. Maps to correct target triple. Fetches latest version from GitHub API, downloads archive, extracts to `~/.local/bin`, makes binary executable. Prints PATH hint if `~/.local/bin` is not in PATH. Fails clearly on unsupported platforms.

## Decisions Made

1. SHA shown **before** the operation status line so context appears above result
2. Separate install/update/remove counters instead of single `succeeded` counter — enables accurate summary breakdown
3. Summary line green on full success, red if any failures — matches cargo's approach
4. `~/.local/bin` as install target — XDG standard, avoids needing sudo
5. POSIX sh for install script — works on minimal systems without bash

## Deviations from Plan

### Notes

Task 2's verbose SHA work was implemented as part of Task 1 execution since both tasks touched `execute_sync_plan`. The final commit for Task 2 added only the doc comment update. This was intentional: the plan itself noted verbose was closely tied to the same function modifications.

**None — plan executed exactly as written.** All must_haves satisfied:
- Colored status words (green installed, yellow updated, red failed) ✓
- Colors auto-disable when not a TTY ✓
- One-line colored summary at end ✓
- --verbose shows per-skill SHA comparisons ✓
- Dry-run colored prefixes (+green, ~yellow, -red) ✓
- GitHub Actions release workflow for macOS ARM + Linux x86_64 ✓
- Install script with OS/arch detection ✓

## Self-Check: PASSED

All created files confirmed present. All task commits confirmed in git history.

| Check | Result |
|-------|--------|
| .github/workflows/release.yml | FOUND |
| install.sh | FOUND |
| src/sync.rs | FOUND |
| src/main.rs | FOUND |
| Cargo.toml | FOUND |
| commit f06c751 (Task 1) | FOUND |
| commit d9514fb (Task 2) | FOUND |
| commit b9ca90e (Task 3) | FOUND |
