# Phase 1: Foundation - Context

**Gathered:** 2026-02-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Crate scaffold, manifest parsing, metadata types, and error infrastructure. Delivers a compiling Rust binary that can parse `.skills.toml`, validate entries, discover manifests from multiple locations, generate a sample config via `--init`, and read/write per-skill metadata atomically. No network calls or git operations — those are Phase 2+.

</domain>

<decisions>
## Implementation Decisions

### --init output style
- Tutorial-style generated `.skills.toml` with full header, usage section, format explanation, and commented examples — self-teaching for new users
- Include 2-3 commented-out example repos like `# "anthropics/claude-skills"` with annotations explaining single-skill vs monorepo
- Global init location: `~/.config/skills-sync/skills.toml` (XDG-style, matches PRD)
- If `.skills.toml` already exists when running `--init`: error with message showing path to existing file, refuse to overwrite

### Error message tone
- Cargo-style structured errors: `error: manifest not found` with `-->` hints and remediation suggestions
- Semantic terminal colors: red=error, yellow=warning, green=success, dim=secondary info. Respect `NO_COLOR` env var
- Default output (without `--verbose`): summary only — actions taken + final count like PRD output examples
- Every error includes remediation hint suggesting what to do next

### Validation behavior
- Strict + helpful: reject invalid entries with specific error (line number, what's wrong, how to fix). No silent skipping
- Empty repos list: warn and exit cleanly (exit 0) — "No repos listed in manifest. Add repos to [skills].repos."
- Whitespace/trailing slashes: auto-trim silently. `" owner/repo "` becomes `"owner/repo"`
- Unknown TOML keys: warn and continue — forward-compatible for future extensions

### Metadata schema
- Include `"schema_version": 1` from day one for painless future migrations
- ISO 8601 timestamps (`"2026-02-28T12:00:00Z"`) — human-readable, standard, sortable
- Fields match PRD exactly: source, subdirectory (null for single-skill), sha, installed_at, updated_at
- Meta files stay visible (not gitignored) — they're part of the managed state

### Claude's Discretion
- Exact Cargo.toml dependency versions (use latest stable per research)
- Module file organization within src/
- Test structure and fixture design
- Atomic write implementation approach (temp file + rename)

</decisions>

<specifics>
## Specific Ideas

- Error output should feel like Cargo — structured, colored, with remediation hints
- The `--init` output should match the PRD example almost exactly — it was carefully designed with inline documentation
- Validation should catch mistakes early but not be hostile — auto-trim whitespace, warn on unknowns

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- None — greenfield project, no existing code

### Established Patterns
- None yet — this phase establishes the foundational patterns

### Integration Points
- `Cargo.toml` will define all dependencies per research STACK.md recommendations
- Module structure established here will be the scaffolding for all subsequent phases

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-foundation*
*Context gathered: 2026-02-28*
