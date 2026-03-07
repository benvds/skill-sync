---
status: testing
phase: 04-cli-ux
source: [04-01-SUMMARY.md, 04-02-SUMMARY.md]
started: 2026-03-02T17:15:00Z
updated: 2026-03-04T12:00:00Z
---

## Current Test

number: 6
name: Colored dry-run output
expected: |
Run `skills-sync --dry-run` in a terminal (not piped) — the `+` prefix is green, `~` is yellow, `-` is red. Warnings show yellow "warning:" prefix.
awaiting: user response

## Tests

### 1. --version flag

expected: Run `skills-sync --version` — prints version matching Cargo.toml (e.g. "skills-sync 0.1.0")
result: pass

### 2. --help flag

expected: Run `skills-sync --help` — shows usage with --dry-run, --force, --verbose, --local, --global flags listed
result: pass

### 3. --global --local mutual exclusivity

expected: Run `skills-sync --global --local` — clap rejects with error before any network calls (something like "cannot be used with")
result: pass

### 4. --dry-run with pending changes

expected: Run `skills-sync --dry-run` when skills need syncing — shows lines prefixed with `+` (install), `~` (update), or `-` (remove), then exits with code 2. Verify with `echo $?` after.
result: pass

### 5. --dry-run when up-to-date

expected: Run `skills-sync --dry-run` when everything is already synced — prints "everything up to date" and exits with code 0
result: pass

### 6. Colored dry-run output

expected: Run `skills-sync --dry-run` in a terminal (not piped) — the `+` prefix is green, `~` is yellow, `-` is red. Warnings show yellow "warning:" prefix.
result: pass

### 7. Colors auto-disable when piped

expected: Run `skills-sync --dry-run | cat` — output has no ANSI escape codes (plain text, no color sequences)
result: pass

### 8. Colored sync output

expected: Run `skills-sync` to perform a sync — status words are colored: "installed:" green, "updated:" yellow, "failed:" red. Summary line is green when all succeed.
result: pass

### 9. --verbose SHA comparisons

expected: Run `skills-sync --verbose` — before each operation, shows SHA details on stderr: install shows `dir -> abcd1234`, update shows `dir old_sha -> new_sha`, skip shows `skip: dir (sha)`
result: pass

### 10. --force auto-confirms orphan removal

expected: With orphaned skills present, run `skills-sync --force` — orphans are removed without interactive prompt (no dialoguer confirmation)
result: [pending]

### 11. Summary line format

expected: After sync, summary reads "sync complete: N installed, N updated, N removed" with separate counts (not a single "succeeded" count)
result: [pending]

### 12. Install script OS detection

expected: Run `sh install.sh` (or read the script) — it detects Darwin-arm64 or Linux-x86_64 via uname, targets ~/.local/bin, and fails clearly on unsupported platforms
result: [pending]

## Summary

total: 12
passed: 5
issues: 0
pending: 7
skipped: 0

## Gaps

[none yet]
