# Feature Research

**Domain:** CLI package/plugin sync tool — agentic skills from GitHub repos
**Researched:** 2026-02-28
**Confidence:** MEDIUM-HIGH (competitors verified via direct repo inspection; CLI UX patterns verified via clig.dev official guide)

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Declarative manifest file | Every sync tool uses one; users think in "desired state" not "commands" | LOW | `.skills.toml` — TOML chosen for comment support (critical for --init docs) |
| `sync` command (idempotent) | Core operation: run once, state matches manifest; re-run is safe | MEDIUM | Must install missing, update outdated, leave current alone |
| Install missing skills | Broken without it — skills in manifest but not on disk are useless | MEDIUM | Shallow clone (`--depth 1`) or sparse checkout for monorepos |
| Update outdated skills | Users expect "sync = latest" behavior for default-branch repos | MEDIUM | SHA comparison: local HEAD vs remote HEAD via GitHub API |
| Orphan detection with prompt | All package managers remove orphans — skipping this feels incomplete | MEDIUM | Detect skills on disk not in manifest; prompt before removal |
| `--dry-run` flag | CLIG standard; users won't trust a sync tool that can't preview | LOW | Show planned changes without executing; critical for trust |
| `--verbose` flag | CLIG standard; expected on any tool that does network operations | LOW | Show per-skill detail: what's being fetched, compared, installed |
| `--force` flag | Skip confirmation prompts for CI/CD automation | LOW | Bypass interactive orphan prompt; run unattended |
| `--init` flag | Standard "bootstrap" pattern (npm init, aps init, sk init) | LOW | Generate commented `.skills.toml` with examples and docs inline |
| Graceful error handling | Network failures, missing repos, rate limits must not corrupt state | MEDIUM | Partial failure should not leave half-installed state; clear error messages |
| Exit codes (0/non-zero) | CLIG requirement; scripts must detect failure | LOW | Zero on success, non-zero on any error |
| Help text (`--help`) | Every CLI tool must have self-documenting help | LOW | Clap auto-generates from struct definitions |
| `--version` flag | Standard; required for debugging and support | LOW | Clap auto-generates |
| Progress feedback during operations | Users need to know something is happening during git clones | LOW | Spinners or progress bars; operations can take 5-30 seconds |
| Single-skill repo detection | Skills can be at root (SKILL.md) or in subdirs — must handle both | MEDIUM | GitHub Trees API call per repo; check for root SKILL.md vs subdirs |
| Monorepo skill detection | Many skill repos are monorepos with multiple skills in subdirs | MEDIUM | One-level-deep scan for dirs containing SKILL.md |
| State tracking metadata | Required for update detection (what was installed, from where, at what SHA) | LOW | `.skills-sync-meta.json` per skill; enables SHA comparison |
| Local and global install targets | Tools like chezmoi, skillshare all support project-local vs global scope | LOW | `--global` flag or manifest-level config; local = `./.claude/skills`, global = `~/.claude/skills` |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Rust single binary, zero deps | Competitors (skills-cli) use Bun/Node; Rust means instant startup, no runtime install, Homebrew distribution | HIGH | The entire point of building in Rust; distributable via `cargo install` or pre-built binaries |
| Manifest config discovery (CWD + global) | Walk up directory tree for project-local `.skills.toml`; fall back to `~/.config/skills-sync/skills.toml` | LOW | Same pattern as git config; feels natural; no explicit `--config` needed in common cases |
| `--config <path>` explicit override | Power users and monorepos need explicit control | LOW | Highest priority in config resolution chain |
| SHA-based update detection via GitHub API | Lighter than full clone to check freshness; one API call per repo | MEDIUM | `GET /repos/{owner}/{repo}/git/refs/heads/{branch}` returns HEAD SHA; compare to stored meta SHA |
| GitHub Trees API for structure detection | One API call per repo maps entire structure; no recursive git operations | MEDIUM | `GET /repos/{owner}/{repo}/git/trees/{branch}?recursive=1`; detect single-skill vs monorepo without cloning |
| Documented sample config via `--init` | Skills-supply and agentic-prompt-sync both have `init` but the config is often sparse; verbose inline comments serve as tutorial | LOW | TOML comments explaining each field make the config self-teaching |
| Sparse checkout for monorepo skill extraction | Lighter than full clone for monorepos; fetches only the target subdirectory | HIGH | `git sparse-checkout set <subdir>` with `--filter=blob:none`; only competitor advantage if they don't support this |
| Claude Code + Cursor rules focused scope | Competitors (skills-supply, skillshare) support 50+ agents; skills-sync is purpose-built for SKILL.md convention | LOW | Simpler model means fewer edge cases; "do one thing well" |
| Honest, structured output | Color-coded: green = installed, yellow = updated, red = error; each skill on its own line | LOW | Referenced in CLIG guidelines and 2025 CLI UX patterns as a differentiator |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Lock file for reproducible installs | Teams want exact version pinning like package-lock.json | Breaks the "always latest" mental model; adds complexity (resolve vs install commands); creates stale-lock merge conflicts; v1 users just want "sync to latest" | Track SHA in per-skill meta for observability; add lock file as v2 feature when there's clear user demand |
| Version/branch pinning in manifest | "I want `obra/superpowers@v1.2.3`" | Always-latest is the v1 value prop; pinning adds resolver complexity, version conflict detection, and semver parsing; skills don't have a stable versioning convention yet | Ship without; add as v2 after users show demand for stability over freshness |
| Watch mode (auto-sync on change) | "Sync automatically when manifest changes" | File watching is OS-specific, resource-intensive, and introduces daemon complexity; skills rarely change mid-session | `skills-sync` is fast enough to run manually; add to shell hooks or git post-merge hooks instead |
| Private repo auth beyond existing credentials | "Support GitHub tokens for private repos" | Auth management is a security surface; token storage, rotation, expiry — all complexity; git/gh credentials already handle this for most users | Delegate to existing git credential helpers and `gh auth`; document this in help text |
| Non-GitHub sources (GitLab, Bitbucket) | "I have skills on GitLab" | Different APIs, different Tree structures, different auth flows; doubles test matrix; skills ecosystem is GitHub-native | Scope v1 to GitHub; the GitHub Trees API approach is the core value; add providers as v2 |
| Skill dependency resolution | "SkillA requires SkillB" | Dependency graphs require a resolver, conflict detection, cycle detection; no established skill dependency convention exists yet | Skills are standalone by design; if a skill needs another skill, the user lists both in manifest |
| Skill validation/linting | "Validate SKILL.md frontmatter before install" | Validation rules are ecosystem-dependent and unstable; linting out-of-date rules breaks installs | Trust repo maintainers; surface errors at install time, not pre-flight |
| Web dashboard / TUI interface | Skillshare v0.16.0 added TUI; seems user-friendly | Doubles maintenance surface; sync tools are used infrequently enough that a CLI is sufficient; TUI complexity doesn't pay off for a tool run once per session | Invest in excellent `--verbose` and `--dry-run` output instead of a visual interface |
| `--json` output | Power users want machine-readable output | Low demand for skills-sync specifically; tool is run interactively, not piped | Provide clean exit codes; add `--json` in v1.x if CI integrations emerge |
| Interactive skill discovery/selection | agentic-prompt-sync has toggle-selection UI for skill repos | Adds dependency on terminal capabilities; better served by `--init` flag that shows a well-documented manifest to edit | `--init` produces commented config; user edits with their editor |

## Feature Dependencies

```
[manifest parsing]
    └──requires──> [TOML + serde deserialization]

[sync command]
    └──requires──> [manifest parsing]
    └──requires──> [GitHub Trees API call] (structure detection)
    └──requires──> [state tracking metadata] (orphan detection, update detection)
    └──requires──> [git shallow clone / sparse checkout] (install)

[update detection]
    └──requires──> [state tracking metadata] (stored SHA)
    └──requires──> [GitHub API] (remote HEAD SHA)

[orphan detection]
    └──requires──> [state tracking metadata] (installed skills list)
    └──requires──> [manifest parsing] (desired skills list)

[--force flag]
    └──enhances──> [orphan detection] (skip removal confirmation)

[--dry-run flag]
    └──enhances──> [sync command] (preview without execute)
    └──conflicts──> [--force flag] (dry-run is read-only, force implies write)

[--verbose flag]
    └──enhances──> [sync command] (per-step detail output)
    └──enhances──> [update detection] (show SHA comparison)

[--init flag]
    ──standalone──> (no sync dependencies; generates template config only)

[local vs global target]
    └──requires──> [manifest discovery] (CWD vs global config)

[monorepo detection]
    └──requires──> [GitHub Trees API call]
    └──requires──> [sparse checkout] (for extraction of single skill dir)

[single-skill detection]
    └──requires──> [GitHub Trees API call]
    └──requires──> [shallow clone] (for full single-skill repo)
```

### Dependency Notes

- **sync requires manifest parsing:** The sync command is entirely driven by what's in the manifest; no manifest = no sync
- **update detection requires state metadata:** Without stored SHA from previous install, we cannot know if a skill is outdated
- **orphan detection requires both metadata and manifest:** Orphans = (installed set) minus (manifest set); needs both to compute the diff
- **--dry-run conflicts with --force:** dry-run is a read-only preview mode; force bypasses prompts on write operations; combining them is a user error that should be caught early
- **monorepo detection requires sparse checkout:** Full clone of a monorepo just to extract one subdirectory wastes bandwidth; sparse checkout is the only viable approach for monorepos

## MVP Definition

### Launch With (v1)

Minimum viable product — what's needed to validate the concept.

- [ ] Parse `.skills.toml` manifest with repo list validation — core contract with user
- [ ] Detect single-skill repos vs monorepos via GitHub Trees API — required before any install
- [ ] Install missing skills (shallow clone / sparse checkout) — the primary value
- [ ] Update outdated skills via SHA comparison — without this, sync is just install
- [ ] Detect orphans and prompt for removal — without this, manifests diverge from disk over time
- [ ] Write `.skills-sync-meta.json` per installed skill — prerequisite for update detection and orphan detection
- [ ] `--dry-run` flag — users won't trust a sync tool that can't preview changes
- [ ] `--force` flag — CI/CD requires non-interactive operation
- [ ] `--verbose` flag — debugging and transparency
- [ ] `--init` flag — bootstrap experience; lower barrier to adoption
- [ ] Graceful error handling (network, rate limits, missing repos) — correctness requirement
- [ ] Local and global target support — project-local vs user-global is day-one requirement
- [ ] Manifest config discovery (CWD + global fallback) — convenience; avoids --config flag in most cases

### Add After Validation (v1.x)

Features to add once core is working.

- [ ] `--json` output flag — trigger: CI integrations request it
- [ ] Explicit `--config <path>` override — trigger: monorepo users need it
- [ ] Improved SHA comparison caching — trigger: rate limit complaints for large manifests
- [ ] Shell completion (zsh, bash, fish) — trigger: power user requests after v1 ships

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] Version/branch pinning (`@v1.2.3`, `@main`) — defer: adds resolver complexity; need evidence users need it
- [ ] Lock file for reproducible installs — defer: conflicts with "always latest" v1 model; ship when teams demand it
- [ ] Private repo auth management — defer: git credential helpers cover most cases; explicit auth = security surface
- [ ] Non-GitHub sources (GitLab, Bitbucket) — defer: GitHub is where the ecosystem lives; add when users request
- [ ] Watch mode — defer: daemon complexity for infrequent operation
- [ ] Skill dependency resolution — defer: no established convention yet

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Manifest parsing + sync command | HIGH | MEDIUM | P1 |
| Install missing skills | HIGH | MEDIUM | P1 |
| Update outdated skills (SHA comparison) | HIGH | MEDIUM | P1 |
| Orphan detection + removal prompt | HIGH | MEDIUM | P1 |
| State tracking metadata per skill | HIGH | LOW | P1 |
| `--dry-run` flag | HIGH | LOW | P1 |
| `--force` flag | MEDIUM | LOW | P1 |
| `--verbose` flag | MEDIUM | LOW | P1 |
| `--init` flag | HIGH | LOW | P1 |
| Graceful error handling | HIGH | MEDIUM | P1 |
| Local vs global target | HIGH | LOW | P1 |
| Monorepo sparse checkout | HIGH | HIGH | P1 |
| GitHub Trees API structure detection | HIGH | MEDIUM | P1 |
| Honest structured output (color, per-skill) | MEDIUM | LOW | P2 |
| Config discovery (CWD + global fallback) | MEDIUM | LOW | P2 |
| Shell completions | LOW | LOW | P2 |
| `--json` output | LOW | LOW | P3 |
| Lock file | LOW | HIGH | P3 |
| Version/branch pinning | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

## Competitor Feature Analysis

| Feature | agentic-prompt-sync (APS) | skills-supply (sk) | skillshare | skills-cli | skills-sync (ours) |
|---------|--------------------------|-------------------|------------|------------|-------------------|
| Language/runtime | Python | Node.js | Go | Bun/Node | **Rust** (single binary) |
| Manifest format | YAML | TOML | YAML | JSON/config | **TOML** (with comments) |
| Sync command | `aps sync` | `sk sync` | `skillshare sync` | `skills sync` | `skills-sync` (default) |
| Dry-run | `--dry-run` | `--dry-run` | Not found | Not found | `--dry-run` |
| Force/non-interactive | `--yes` | `--non-interactive` | Not found | Not found | `--force` |
| Orphan removal | "backups" on conflict | "safely removes stale" | Not found | Not found | Prompt with `--force` bypass |
| Lock file | Yes | No | No | No | **No (v2)** |
| Version pinning | No | Yes (tag/branch/rev) | Not found | No | **No (v1), v2** |
| Multi-agent targets | No | Yes (5 agents) | Yes (50+ agents) | Yes (many agents) | **No — Claude-focused only** |
| Monorepo support | include filter | Auto-detect SKILL.md subdirs | Not found | `--subdir` flag | Sparse checkout, 1-level deep |
| Status command | `aps status` | Not found | Not found | `skills status` | Via `--verbose` on sync |
| Doctor/validate | `aps validate` | Not found | Not found | `skills doctor` | Graceful errors + clear messages |
| Init command | `aps init` | `sk init` | `skillshare init` | Not explicit | `--init` flag |
| Distribution | pip/pipx | npm | Binary releases | bun/npm global | **Homebrew, cargo, binaries** |
| GitHub-only scope | No (filesystem too) | No (multiple git remotes) | No (5 git hosts) | No (multi-host) | **Yes — GitHub-focused v1** |

**Our differentiation:** Rust single binary (no runtime), GitHub-focused simplicity, TOML with rich inline docs, Claude Code native (SKILL.md convention), Homebrew-distributable. We don't try to be a universal multi-agent sync tool — we do one job excellently.

## Sources

- [chezmoi comparison table](https://www.chezmoi.io/comparison-table/) — dotfiles manager feature benchmark (MEDIUM confidence, official docs)
- [Command Line Interface Guidelines — clig.dev](https://clig.dev/) — authoritative CLI UX standard (HIGH confidence, official reference)
- [lazy.nvim GitHub](https://github.com/folke/lazy.nvim) — plugin manager patterns (lockfile, orphan cleanup, dry-run) (HIGH confidence, official repo)
- [agentic-prompt-sync GitHub](https://github.com/westonplatter/agentic-prompt-sync) — direct competitor (HIGH confidence, official repo)
- [skills-supply GitHub](https://github.com/803/skills-supply) — direct competitor with TOML manifest (HIGH confidence, official repo)
- [skillshare GitHub](https://github.com/runkids/skillshare) — direct competitor (HIGH confidence, official repo)
- [skills-cli GitHub](https://github.com/dhruvwill/skills-cli) — direct competitor (HIGH confidence, official repo)
- [awesome-claude-skills](https://github.com/travisvn/awesome-claude-skills) — SKILL.md structure reference (MEDIUM confidence)
- [awesome-claude-code](https://github.com/hesreallyhim/awesome-claude-code) — Claude skills ecosystem structure (MEDIUM confidence)
- [Evil Martians CLI UX patterns](https://evilmartians.com/chronicles/cli-ux-best-practices-3-patterns-for-improving-progress-displays) — progress display patterns (MEDIUM confidence)
- [GitHub sparse checkout blog](https://github.blog/open-source/git/bring-your-monorepo-down-to-size-with-sparse-checkout/) — sparse checkout for monorepos (HIGH confidence, official GitHub blog)

---
*Feature research for: CLI package/plugin sync tool (agentic skills from GitHub repos)*
*Researched: 2026-02-28*
