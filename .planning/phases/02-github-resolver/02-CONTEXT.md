# Phase 2: GitHub + Resolver - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Call the GitHub Trees API to detect repo structure (single-skill vs monorepo), fetch HEAD SHAs, and produce a correct sync plan describing what needs to be installed, updated, removed, or skipped. This phase delivers the API client and pure resolver logic — no disk operations or CLI output formatting.

</domain>

<decisions>
## Implementation Decisions

### GitHub authentication
- Token discovery priority: GITHUB_TOKEN env var > `gh auth token` CLI > unauthenticated
- Silent by default — auth source only shown with --verbose
- Warn once when running unauthenticated: hint about 60 req/hr limit and how to authenticate
- Private repos work when a token is present; clear error if private repo accessed without token

### Repo detection
- Detection details (single-skill vs monorepo, skill counts) only shown with --verbose
- No-SKILL.md warning printed inline per repo during the run: "warning: user/repo — no SKILL.md found, skipping"
- Root SKILL.md wins: if SKILL.md exists at repo root, treat as single-skill regardless of subdirectories. Monorepo detection only when root has no SKILL.md
- No rename detection for monorepo subdirectories — renamed skill = orphan remove + fresh install

### Sync plan shape
- Four action types: Install (new skill), Update (SHA changed), Remove (orphan), Skip (unchanged)
- Resolver returns a pure data struct (e.g., Vec<SyncAction>) — no display logic in the resolver
- Sequential repo processing (no async/parallel) — can be optimized later
- Skip actions tracked explicitly so downstream consumers can report "N skills up to date"

### Error & rate limit behavior
- Rate limit hit: fail immediately with quota info (remaining count, reset time) and auth hint. No retry
- Per-repo API failure (404, network error): skip with inline warning, continue processing remaining repos
- Failed repos summarized at end of run
- Exit code non-zero if any repo failed, even if others succeeded (CI-friendly)
- Reactive rate limit handling — no pre-check of /rate_limit endpoint

### Claude's Discretion
- Sync action struct design (what fields each action carries beyond the basics)
- HTTP client choice (sync vs async, library selection)
- Internal error type design for GitHub API errors (extending existing miette/thiserror pattern)
- Trees API response parsing approach

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Manifest` struct (manifest.rs): Provides parsed `repos: Vec<String>` in owner/repo format — direct input for API calls
- `SkillMeta` (metadata.rs): Has `source`, `sha`, `subdirectory`, `installed_at`, `updated_at` fields — resolver compares remote SHA against local `sha` field
- `write_metadata_atomic` / `read_metadata` (metadata.rs): Atomic file I/O for metadata — resolver reads existing metadata to determine update vs skip
- miette + thiserror error pattern (error.rs): Established diagnostic error pattern with codes, help text, and source spans

### Established Patterns
- clap derive for CLI args (cli.rs) — --verbose flag already exists for gating detection output
- serde Serialize/Deserialize for data types — sync actions should follow this pattern
- LazyLock for compiled regex (manifest.rs) — can reuse pattern for any static data
- No async runtime — project is synchronous, sync HTTP client preferred unless compelling reason for async

### Integration Points
- `main.rs:run()` currently: parse CLI → discover manifest → parse manifest → done. Phase 2 adds: → resolve auth → call GitHub API per repo → build sync plan → return plan
- `Cli` struct (cli.rs): Already has `verbose: bool` — used to gate detection details and auth source display
- `SkillsSyncError` enum (error.rs): Needs new variants for GitHub API errors, rate limits, repo not found, etc.

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-github-resolver*
*Context gathered: 2026-02-28*
