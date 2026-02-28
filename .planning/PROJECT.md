# skills-sync

## What This Is

A command-line tool that manages agentic skills (Claude Skills, Cursor rules, etc.) by syncing them from GitHub repositories to a local or global skills directory. It reads a declarative `.skills.toml` manifest file, resolves each listed repository's structure, and ensures the local skills directory matches the desired state — installing missing skills, updating outdated ones, and detecting orphans.

## Core Value

Declarative, zero-friction skill synchronization: run one command and your skills directory matches your manifest.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Parse `.skills.toml` manifest with repo list validation
- [ ] Detect single-skill repos (SKILL.md at root) vs monorepos (subdirectories with SKILL.md)
- [ ] Sync skills to local (`./.claude/skills`) or global (`~/.claude/skills`) target
- [ ] Install missing skills via shallow git clone
- [ ] Update outdated skills by comparing local vs remote HEAD SHA
- [ ] Detect and prompt for orphaned skill removal
- [ ] `--init` flag generates documented sample `.skills.toml`
- [ ] `--dry-run` shows planned changes without executing
- [ ] `--force` skips confirmation prompts
- [ ] `--verbose` shows detailed output
- [ ] Write `.skills-sync-meta.json` per installed skill for state tracking
- [ ] Graceful error handling (missing repos, rate limits, network failures)

### Out of Scope

- Version/branch pinning — always syncs to latest default branch (v2)
- Private repo auth beyond existing git/gh credentials — (v2)
- Non-GitHub sources (GitLab, Bitbucket) — (v2)
- Skill dependency resolution — not needed for v1
- Skill validation/linting — not needed for v1
- Lock file for reproducible setups — (v2)
- Watch mode — (v2)

## Context

- Built in Rust for single-binary distribution, proper error handling, and cross-platform support
- Key crates: `clap` (CLI), `toml`+`serde` (parsing), `reqwest` (GitHub API), `dialoguer` (prompts), `indicatif` (progress), `anyhow` (errors), `dirs` (paths)
- GitHub Trees API (`GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1`) for efficient repo structure detection (one call per repo)
- Manifest discovery: CWD `.skills.toml` (local), `~/.config/skills-sync/skills.toml` (global), or explicit `--config <path>`
- Shallow clones (`--depth 1`) for single-skill repos; sparse checkout for monorepo skill extraction
- Distribution via Homebrew, `cargo install`, and pre-built binaries (x86_64 Linux, x86_64/aarch64 macOS)

## Constraints

- **Language**: Rust — single binary, no runtime dependencies, cross-platform
- **API**: GitHub-only for v1 — uses GitHub Trees API and git protocols
- **Depth**: Only checks one level deep for monorepo skills — deeper nesting is an anti-pattern
- **Updates**: Always syncs to latest default branch HEAD — no version pinning in v1

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| TOML for manifest format | Human-readable, supports comments (critical for --init docs), clear spec, good Rust support. JSON lacks comments, YAML is ambiguous | — Pending |
| Rust over shell script | TOML parsing, JSON API interaction, state management, cross-platform needs exceed bash capabilities | — Pending |
| Shallow clones over tarball download | Middle ground — lighter than full clone, easier updates than tarballs | — Pending |
| Per-skill metadata files | Simpler to manage than central file, makes orphan detection straightforward | — Pending |
| One-level-deep monorepo detection | Fast, predictable traversal; deep nesting is an anti-pattern | — Pending |

---
*Last updated: 2026-02-28 after initialization*
