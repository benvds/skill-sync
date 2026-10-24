# skills-sync

Declarative Agent Skills synchronization for Codex and other agents that support the [Agent Skills specification](https://agentskills.io/specification). One command, your skills directory matches your manifest.

## Quick Start

1. Install skills-sync (see [Installation](#installation))
2. Run `skills-sync --init` to generate a `.skills.toml` manifest
3. Edit `.skills.toml` to list the skill repos you want
4. Run `skills-sync` to sync

Example output on first sync:

```
+ anthropics/skills/skills/skill-creator (install)
+ github/awesome-copilot/skills/github-issues (install)
sync plan: 2 install, 0 update, 0 remove, 0 up-to-date
  installing anthropics/skills/skills/skill-creator...
  installing github/awesome-copilot/skills/github-issues...
done: 2 installed, 0 updated, 0 removed
```

## Installation

### From source

```sh
cargo install --path .
```

### From release

```sh
curl -fsSL https://raw.githubusercontent.com/OWNER/skills-sync/main/install.sh | sh
```

> Note: Replace `OWNER` with the actual GitHub owner before the first release.

## Manifest Format

The manifest is a TOML file (`.skills.toml` by default; `skills.toml` is also discovered). It lists GitHub repos to sync skills from.

### Single-skill repo

A repo that has a `SKILL.md` at the repository root:

```toml
[skills]
repos = [
    "owner/single-skill-repo",
]
```

skills-sync detects the `SKILL.md` at the root and installs the whole repo as one skill.

### Monorepo (auto-detect)

A repo that contains multiple skill subdirectories, each with their own `SKILL.md`:

```toml
[skills]
repos = [
    "owner/multi-skill-monorepo",
]
```

skills-sync scans the repository tree and installs each subdirectory that contains a `SKILL.md` as a separate skill.

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

## Where skills are installed

Automatic targets are based on the **manifest directory**, even when `--config` is run from elsewhere:

| Manifest directory | Skills destination |
|---|---|
| Project root | `<project>/.agents/skills/` |
| `.agents`, `.codex` or `.claude` | Its `skills/` subdirectory |
| Any directory already named `skills` | That directory, directly |
| `~/.agents/skills/` | `~/.agents/skills/` |
| `~/.codex/skills/` | `~/.codex/skills/` |
| Standalone configuration directory | Beside the manifest and lock file |

A directory named `skills` takes precedence, even if it is itself a Git repository. Project detection recognizes `.git` directories and worktree files, `.hg`, agent directories/instructions, and common build manifests such as `Cargo.toml`, `package.json`, `pyproject.toml` and `go.mod`. A new project without these markers can use `--target .agents/skills` or add an `AGENTS.md` file. Symlinked directories are resolved before detection.

The lock file always stays next to its manifest (`.skills.toml` → `.skills.lock`, `skills.toml` → `skills.lock`). No nested `.claude/skills` directory is created. Output is written to the terminal; there is no separate log file.

```sh
# Shared user skills
mkdir -p ~/.agents/skills
cd ~/.agents/skills
skills-sync --init
# Edit .skills.toml, then:
skills-sync

# Codex skills, without creating skills/skills or .claude/skills
mkdir -p ~/.codex/skills
cd ~/.codex/skills
skills-sync --init
skills-sync

# From a project root
skills-sync --init
skills-sync                       # installs to .agents/skills

# Explicit destination (relative to the working directory)
skills-sync --config /path/to/skills.toml --target ~/.codex/skills

# Global mode has its own manifest by default
skills-sync --global --init       # creates ~/.agents/skills/.skills.toml
skills-sync --global
```

Discovery checks `.skills.toml`, then `skills.toml`, in the current directory. It falls back to the platform config directory's `skills-sync/skills.toml` (for example `~/.config/skills-sync/skills.toml` on Linux). An explicit `--config` takes precedence. Use separate manifests for independently managed collections. If the same manifest is pointed at a new target, missing skills are installed there instead of being incorrectly treated as up to date.

Each installed directory uses the `name` from its `SKILL.md` YAML frontmatter, as required by the Agent Skills specification. Required names and descriptions are checked before installation; references, scripts and assets are preserved. Name collisions fail without overwriting existing skills. Existing locks without a directory field remain readable; legacy `owner--repo` installations in the selected target migrate to their declared skill names on the next sync. Skills in an old `.claude/skills` target are not automatically moved or deleted; select that target explicitly if it should still be managed.

## CLI Reference

| Flag | Description |
|------|-------------|
| `--init` | Generate a sample manifest; honors `--config` and `--global` |
| `--config <PATH>` | Manifest path; automatic targets are relative to this file |
| `--dry-run` | Show planned changes without executing; exits 2 if changes pending, 0 if up-to-date |
| `--verbose` | Show detailed output including SHA comparisons and git command output |
| `--global` | Use `~/.agents/skills/` and its manifest |
| `--target <PATH>` | Explicit skills directory, e.g. `~/.codex/skills/`; conflicts with `--global` |
| `--local` | Use automatic target detection (default; conflicts with `--global`) |
| `--force` | Skip confirmation prompts (useful for CI) |
| `--help` | Print help |
| `--version` | Print version |

## How It Works

skills-sync reads your manifest and queries the GitHub Trees API to detect which skills are present in each listed repository. It compares remote tree SHAs against the local lock file (`.skills.lock`) to determine what needs to be installed, updated, or removed. Skills are fetched via sparse git checkout so only the relevant files are downloaded. The lock file tracks installed state and is updated atomically after each operation.

## Authentication

skills-sync works unauthenticated but is subject to GitHub's 60 requests/hour rate limit. For development or large manifests, use an authenticated token for 5,000 requests/hour:

```sh
export GITHUB_TOKEN=ghp_your_token_here
```

skills-sync also automatically uses the `gh` CLI token if you have [GitHub CLI](https://cli.github.com/) installed and are logged in:

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

The automated tests include local Git install/update/remove tests for every target layout and CLI path checks. These exercise the portable filesystem layout without launching a live AI agent. For a networked end-to-end manual smoke test, run:

```sh
./scripts/test-sync.sh
```

This exercises init, dry-run, sync, and idempotent re-sync in a temporary directory with automatic cleanup.
