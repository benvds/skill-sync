# skill-sync

Declarative skill synchronization for Claude Code. One command, your skills directory matches your manifest.

## Quick Start

1. Install skill-sync (see [Installation](#installation))
2. Run `skill-sync --init` to generate a `.skills.toml` manifest
3. Edit `.skills.toml` to list the skill repos you want
4. Run `skill-sync` to sync

Example output on first sync:

```
+ anthropics/skills/skills/skill-creator (install)
+ github/awesome-copilot/skills/gh-cli (install)
sync plan: 2 install, 0 update, 0 remove, 0 up-to-date
  installing anthropics/skills/skills/skill-creator...
  installing github/awesome-copilot/skills/gh-cli...
done: 2 installed, 0 updated, 0 removed
```

## Installation

### From source

```sh
cargo install --path .
```

### From release

```sh
curl -fsSL https://raw.githubusercontent.com/benvds/skill-sync/main/install.sh | sh
```

## Manifest Format

The manifest is a TOML file (`.skills.toml` by default). It lists GitHub repos to sync skills from.

### Single-skill repo

A repo that has a `SKILL.md` at the repository root:

```toml
[skills]
repos = [
    "owner/single-skill-repo",
]
```

skill-sync detects the `SKILL.md` at the root and installs the whole repo as one skill.

### Monorepo (auto-detect)

A repo that contains multiple skill subdirectories, each with their own `SKILL.md`:

```toml
[skills]
repos = [
    "owner/multi-skill-monorepo",
]
```

skill-sync scans the repository tree and installs each subdirectory that contains a `SKILL.md` as a separate skill.

### Subdirectory pin

Install a specific skill from a monorepo without pulling in all skills from that repo:

```toml
[skills]
repos = [
    "anthropics/skills/skills/skill-creator",
    "anthropics/skills/skills/pdf",
]
```

The format is `"owner/repo/path/to/skill"` — the third segment and beyond are treated as the subdirectory path within the repo.

### Mixing layouts

All three formats can be combined in one manifest:

```toml
[skills]
repos = [
    "owner/single-skill-repo",                    # single-skill (auto-detect)
    "owner/multi-skill-collection",               # monorepo (all skills, auto-detect)
    "anthropics/skills/skills/skill-creator",     # specific skill from a monorepo
]
```

See the [examples/](examples/) directory for copy-paste starting points:

- [examples/minimal.skills.toml](examples/minimal.skills.toml) — single skill, minimal config
- [examples/monorepo.skills.toml](examples/monorepo.skills.toml) — monorepo and subdirectory pin
- [examples/mixed.skills.toml](examples/mixed.skills.toml) — all three layouts combined

## CLI Reference

| Flag | Description |
|------|-------------|
| `--init` | Generate a sample `.skills.toml` in the current directory |
| `--config <PATH>` | Path to manifest file (default: auto-discover `.skills.toml`) |
| `--dry-run` | Show planned changes without executing; exits 2 if changes pending, 0 if up-to-date |
| `--verbose` | Show detailed output including SHA comparisons and git command output |
| `--global` | Sync to `~/.claude/skills/` instead of `.claude/skills/` |
| `--local` | Sync to `.claude/skills/` (default; mutually exclusive with `--global`) |
| `--force` | Skip confirmation prompts (useful for CI) |
| `--help` | Print help |
| `--version` | Print version |

## How It Works

skill-sync reads your manifest and queries the GitHub Trees API to detect which skills are present in each listed repository. It compares remote tree SHAs against the local lock file (`.skills.lock`) to determine what needs to be installed, updated, or removed. Skills are fetched via sparse git checkout so only the relevant files are downloaded. The lock file tracks installed state and is updated atomically after each operation.

## Authentication

skill-sync works unauthenticated but is subject to GitHub's 60 requests/hour rate limit. For development or large manifests, use an authenticated token for 5,000 requests/hour:

```sh
export GITHUB_TOKEN=ghp_your_token_here
```

skill-sync also automatically uses the `gh` CLI token if you have [GitHub CLI](https://cli.github.com/) installed and are logged in:

```sh
gh auth login
```

The unauthenticated limit is usually sufficient for small manifests with infrequent syncs.

## Examples

See the [examples/](examples/) directory:

- **[minimal.skills.toml](examples/minimal.skills.toml)** — Minimal manifest with a single skill. Good starting point.
- **[monorepo.skills.toml](examples/monorepo.skills.toml)** — Demonstrates monorepo auto-detection and subdirectory pinning.
- **[mixed.skills.toml](examples/mixed.skills.toml)** — Combines single-skill, monorepo, and subdirectory-pinned entries in one manifest.

## Development

```sh
# Build
cargo build

# Run tests
cargo test

# Dry-run with the local manifest
cargo run -- --dry-run

# Sync with verbose output
cargo run -- --verbose

# Test with a specific manifest
cargo run -- --dry-run --config examples/mixed.skills.toml
```

For a full end-to-end manual smoke test, run:

```sh
./scripts/test-sync.sh
```

This exercises init, dry-run, sync, and idempotent re-sync in a temporary directory with automatic cleanup.
