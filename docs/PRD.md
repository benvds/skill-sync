# Product Requirements Document: `skill-sync`

**Version:** 0.1.0
**Author:** Ben / Lean Digital
**Date:** 2026-02-28
**Status:** Draft

## Summary

`skill-sync` is a command-line tool that manages agentic skills (e.g. Claude Skills, Cursor rules, or similar AI coding assistant configurations) by syncing them from GitHub repositories to a local or global skills directory. It reads a declarative `.skills.toml` manifest file, resolves each listed repository, and ensures the local skills directory matches the desired state.

## Problem

Managing agentic skills across projects is manual and error-prone. You clone repos, copy directories, forget to update them, and end up with stale or orphaned skills scattered around. There's no standard way to declare "these are the skills I want" and keep them in sync.

## Goals

- Declarative skill management via a single manifest file
- Automatic install, update, and orphan detection
- Support both global (`~/.claude/skills`) and local (`./.claude/skills`) targets
- Smart GitHub repository traversal to detect single-skill repos vs. multi-skill monorepos
- Zero-config updates: outdated skills always get pulled to latest

## Non-Goals (v1)

- Version pinning (always syncs to latest default branch)
- Non-GitHub sources (GitLab, Bitbucket)
- Skill dependency resolution between skills
- Skill validation or linting

---

## File Format: `.skills.toml`

### Why TOML?

TOML is the right fit here. It's human-readable, supports comments natively (critical for the `--init` documentation), has a clear spec, and is well-supported in Rust. JSON lacks comments. YAML is ambiguous. A plain text list can't scale to future config options like branch pinning or auth tokens.

### Schema

```toml
# .skills.toml - Skills manifest for skill-sync
# Docs: https://github.com/benvds/skill-sync

# Each skill entry is a GitHub repository.
# skill-sync will inspect the repo to determine if it's:
#   1. A single skill (SKILL.md at root) → installed as-is
#   2. A monorepo with multiple skills (subdirectories containing SKILL.md) → each extracted

[skills]
repos = [
  "anthropics/claude-skills",           # monorepo: extracts each subdirectory skill
  "user/my-custom-skill",               # single skill repo
  "org/shared-skills",                  # auto-detected based on structure
]

# Future: per-repo overrides
# [skills.overrides."user/my-custom-skill"]
# branch = "develop"
# subdirectory = "experimental"
```

### File Discovery

`skill-sync` looks for the manifest in this order:

1. `.skills.toml` in the current directory (for `--local`)
2. `~/.config/skill-sync/skills.toml` (for `--global`)
3. Explicit path via `--config <path>`

---

## CLI Interface

### Usage

```
skill-sync [FLAGS] [OPTIONS]

FLAGS:
    --global          Sync skills to ~/.claude/skills
    --local           Sync skills to ./.claude/skills (default)
    --init            Create a sample .skills.toml with documentation
    --dry-run         Show what would change without doing it
    --force           Skip confirmation prompts
    --verbose         Show detailed output
    -h, --help        Print help
    -V, --version     Print version

OPTIONS:
    --config <PATH>   Path to manifest file (overrides default discovery)
```

### Flag Behavior

`--global` and `--local` are mutually exclusive. If neither is provided, default to `--local`.

`--init` creates a `.skills.toml` file in the current directory (or `~/.config/skill-sync/skills.toml` when combined with `--global`). The generated file contains commented-out documentation explaining the format, plus a few example entries.

---

## Core Logic

### 1. Parse Manifest

Read and parse the `.skills.toml` file. Extract the list of repository references from `skills.repos`.

Each entry is a GitHub `owner/repo` string. Validation rules:
- Must match pattern `^[a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+$`
- Duplicates are rejected with an error

### 2. Resolve Repository Structure

For each repository, determine whether it contains a single skill or multiple nested skills. This is the trickiest part of the tool.

**Detection algorithm:**

```
For repo in manifest:
  1. Fetch repo root contents via GitHub API (GET /repos/{owner}/{repo}/contents)
  2. Check if SKILL.md exists at root
     → YES: this is a single-skill repo
       - Skill name = repo name (or from SKILL.md front matter if present)
       - Install entire repo contents as one skill directory
     → NO: check for subdirectories containing SKILL.md
       - For each top-level directory, check if it contains a SKILL.md
       - Each matching subdirectory is a separate skill
       - Skill name = subdirectory name
  3. If no SKILL.md found anywhere at root or one level deep:
     → Warn the user and skip this repo
```

**Why only one level deep?** Keeps traversal fast and predictable. Deeply nested skills structures are an anti-pattern. If someone needs deeper nesting, they should restructure their repo.

**API efficiency:** Use the GitHub Trees API (`GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1`) instead of multiple contents calls. One API call per repo gives the full file tree.

### 3. Determine Target Directory

Based on the flag:

| Flag | Target directory |
|------|-----------------|
| `--global` | `~/.claude/skills/` |
| `--local` (default) | `./.claude/skills/` |

If the target directory doesn't exist, create it (including parent directories).

### 4. Compute Diff

Compare the desired state (manifest) against the current state (what's installed):

| State | Action |
|-------|--------|
| **Missing skill** (in manifest, not installed) | Clone/copy to target |
| **Outdated skill** (installed, but behind remote HEAD) | Pull latest / replace |
| **Orphaned skill** (installed, not in manifest) | List and prompt for deletion |
| **Up-to-date skill** (installed, matches remote HEAD) | Skip (log in verbose mode) |

**Freshness check:** Compare the local HEAD SHA against the remote default branch HEAD SHA. Use `git ls-remote` for the remote check (no clone needed) and read `.git/HEAD` or a stored `.skill-sync-meta.json` file in each skill directory for the local state.

### 5. Execute Sync

#### Installing a Skill

For a single-skill repo:
```
git clone --depth 1 https://github.com/{owner}/{repo}.git {target}/{skill-name}
```

For a monorepo with nested skills, use sparse checkout:
```
git clone --depth 1 --filter=blob:none --sparse https://github.com/{owner}/{repo}.git /tmp/skill-sync-{repo}
cd /tmp/skill-sync-{repo}
git sparse-checkout set {subdirectory}
cp -r {subdirectory} {target}/{skill-name}
```

After install, write metadata:
```json
// {target}/{skill-name}/.skill-sync-meta.json
{
  "source": "owner/repo",
  "subdirectory": null,
  "sha": "abc123...",
  "installed_at": "2026-02-28T12:00:00Z",
  "updated_at": "2026-02-28T12:00:00Z"
}
```

#### Updating a Skill

For single-skill repos that were cloned directly:
```
cd {target}/{skill-name} && git pull --ff-only
```

For extracted monorepo skills: re-clone the monorepo to temp, copy the updated subdirectory over the existing one, update metadata.

#### Removing Orphaned Skills

List orphaned skills with their source info (if metadata exists) and prompt:

```
The following skills are installed but not in your manifest:

  • old-skill (was from: user/deprecated-repo)
  • experimental-thing (unknown source)

Remove these skills? [y/N/select]
```

With `--force`, skip the prompt and remove them. With `--dry-run`, just list them.

---

## The `--init` Output

Running `skill-sync --init` creates:

```toml
# ============================================================================
# .skills.toml - Skills manifest for skill-sync
# ============================================================================
#
# This file declares which agentic skills to sync to your skills directory.
# Run `skill-sync` to install and update all listed skills.
#
# USAGE:
#   skill-sync --local      Sync to ./.claude/skills (default)
#   skill-sync --global     Sync to ~/.claude/skills
#   skill-sync --dry-run    Preview changes without applying
#
# REPOSITORY FORMAT:
#   Each entry is a GitHub "owner/repo" string.
#   skill-sync auto-detects the repo structure:
#
#   - If the repo has a SKILL.md at root → installed as a single skill
#   - If subdirectories have SKILL.md files → each is extracted as a skill
#   - Repos without any SKILL.md are skipped with a warning
#
# EXAMPLES:
#   "anthropics/claude-skills"    # monorepo with multiple skills
#   "yourname/my-custom-skill"    # single skill repo
#
# ============================================================================

[skills]
repos = [
  # Add your skill repositories here:
  # "owner/repo-name",
]
```

---

## Metadata and State Tracking

Each installed skill gets a `.skill-sync-meta.json` file. This is how `skill-sync` tracks what's installed, where it came from, and whether it's current.

The metadata file is small, doesn't interfere with the skill itself, and makes orphan detection and update checks possible without hitting the GitHub API for every installed skill.

---

## Error Handling

| Scenario | Behavior |
|----------|----------|
| No manifest file found | Error with hint to run `--init` |
| Invalid TOML syntax | Error with line number and description |
| Repository doesn't exist or is inaccessible | Warn and skip, continue with other repos |
| GitHub API rate limit hit | Error with retry suggestion, show remaining quota |
| No SKILL.md found in repo | Warn and skip |
| Network failure mid-sync | Partial sync is fine; re-running resolves it |
| Target directory not writable | Error with permission fix suggestion |
| Conflicting flags (`--global` + `--local`) | Error immediately |

---

## Implementation Language: Rust

### Why not a shell script?

The initial thought was "bash if simple enough." It's not. Here's what tips it into Rust territory:

- **TOML parsing.** No native TOML parser in bash. You'd need `python -c` or `dasel` or `yq`, adding fragile dependencies.
- **GitHub API interaction.** Parsing JSON tree responses, handling pagination, rate limiting, error codes. `curl` + `jq` works but gets brittle fast.
- **State management.** Reading/writing JSON metadata files per skill, comparing SHAs.
- **Interactive prompts.** Colored output, selection menus for orphan deletion.
- **Cross-platform.** Bash scripts behave differently on macOS vs Linux (GNU vs BSD coreutils).

Rust gives you a single binary with no runtime dependencies, proper error handling, and the `toml`, `serde`, `reqwest`, and `clap` crates handle the heavy lifting.

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `toml` + `serde` | Manifest parsing and metadata serialization |
| `reqwest` | GitHub API calls (async with tokio) |
| `dialoguer` | Interactive prompts (orphan deletion confirmation) |
| `indicatif` | Progress bars for clone/update operations |
| `anyhow` | Error handling with context |
| `dirs` | Cross-platform home directory resolution |

### Project Structure

```
skill-sync/
├── Cargo.toml
├── README.md
├── LICENSE
├── src/
│   ├── main.rs              # Entry point, CLI setup
│   ├── cli.rs               # Clap argument definitions
│   ├── config.rs            # Manifest parsing and discovery
│   ├── github.rs            # GitHub API client, repo structure detection
│   ├── resolver.rs          # Diff computation (missing/outdated/orphaned)
│   ├── sync.rs              # Install, update, remove operations
│   ├── metadata.rs          # .skill-sync-meta.json read/write
│   └── init.rs              # --init template generation
└── tests/
    ├── config_test.rs
    ├── resolver_test.rs
    └── fixtures/
        └── sample_skills.toml
```

---

## Output Examples

### Normal Sync

```
$ skill-sync --local

Reading .skills.toml...
Resolving 3 repositories...

  ✓ anthropics/claude-skills (monorepo: 5 skills detected)
  ✓ user/my-custom-skill (single skill)
  ✗ org/broken-repo (not found, skipping)

Syncing to ./.claude/skills/

  ↓ Installing: docx (from anthropics/claude-skills)
  ↓ Installing: pdf (from anthropics/claude-skills)
  ↑ Updating:  xlsx (from anthropics/claude-skills) [3 commits behind]
  ✓ Up to date: my-custom-skill
  ↓ Installing: pptx (from anthropics/claude-skills)

Orphaned skills found:
  • old-deprecated-skill (was from: someone/old-repo)

Remove orphaned skills? [y/N]: y
  ✗ Removed: old-deprecated-skill

Done. 4 installed, 1 updated, 1 removed, 1 skipped.
```

### Dry Run

```
$ skill-sync --dry-run

Would sync to ./.claude/skills/:
  [install]  docx (from anthropics/claude-skills)
  [install]  pdf (from anthropics/claude-skills)
  [update]   xlsx (from anthropics/claude-skills)
  [skip]     my-custom-skill (up to date)
  [remove?]  old-deprecated-skill (orphaned)

No changes made (dry run).
```

---

## Distribution

### Homebrew

```ruby
class SkillSync < Formula
  desc "Sync agentic skills from GitHub repositories"
  homepage "https://github.com/benvds/skill-sync"
  url "https://github.com/benvds/skill-sync/releases/download/v0.1.0/skill-sync-v0.1.0.tar.gz"
  sha256 "..."

  def install
    bin.install "skill-sync"
  end
end
```

### Cargo

```
cargo install skill-sync
```

### Pre-built Binaries

GitHub Releases with binaries for:
- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

---

## Future Considerations (v2+)

These are out of scope for v1 but worth designing around:

- **Branch/tag pinning:** `"owner/repo@v1.2.0"` or `"owner/repo#develop"` syntax in the manifest
- **Private repo auth:** GitHub token configuration for private repos
- **GitLab/Bitbucket support:** Abstract the git hosting provider
- **Skill validation:** Lint SKILL.md files for required fields
- **Lock file:** `.skills-lock.toml` to pin exact SHAs for reproducible setups
- **Skill templates:** `skill-sync new <name>` to scaffold a new skill repo
- **Watch mode:** `skill-sync --watch` to auto-sync on manifest changes
- **Workspace support:** Multiple `.skills.toml` files in a monorepo, each syncing to their own target

---

## Open Questions

1. **Should `--local` be truly local to CWD or follow git root?** Finding the nearest `.git` parent would be more intuitive for monorepo projects, but adds complexity.

2. **Git clone vs. GitHub API download?** Cloning preserves `.git` for easy updates but uses more disk. Downloading tarballs is lighter but makes updates harder. Current design uses shallow clones as a middle ground.

3. **How to handle skills with the same name from different repos?** Prefix with repo name (`anthropics-docx` vs `myorg-docx`)? Or error on collision?

4. **Should metadata file be `.skill-sync-meta.json` or store everything in one central file?** Per-skill metadata is simpler to manage but adds a file to each skill directory.
