# Phase 3: Sync Engine - Context

**Gathered:** 2026-03-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Execute sync actions on disk — install, update, and remove skills while leaving no corrupted state on network failure or interruption. Targets local (`./.claude/skills/`) or global (`~/.claude/skills/`) directories. CLI flags, output polish, and UX are Phase 4.

</domain>

<decisions>
## Implementation Decisions

### Directory layout
- Single-skill repos named `owner--repo` (e.g., `.claude/skills/anthropics--claude-skills/`)
- Monorepo skills named `owner--repo--subdir` (e.g., `.claude/skills/anthropics--monorepo--python/`)
- Flat structure — no nested owner directories
- Metadata file (`.skills-sync-meta.json`) lives inside the skill directory — self-contained
- Monorepo sparse checkout extracts subdirectory contents only — no redundant nesting inside the skill dir

### Orphan removal flow
- Batch confirmation — list all orphans, then one `Y/n` prompt
- Show name + source only per orphan (minimal, clean)
- Delete permanently on confirmation — skills are always recoverable by re-adding to manifest
- Warn every run if orphans exist (no suppression state to track)

### Update behavior
- Delete and re-clone — remove old directory, shallow clone fresh. Simple, deterministic, avoids git state issues
- No local modification detection — skills are managed artifacts, local edits not expected to survive
- Preserve `installed_at` timestamp from original install; update `updated_at` and `sha`
- Sequential processing — one skill at a time. Good enough for typical manifest sizes

### Failure recovery
- Clone to temp directory in same parent, atomic rename to final path on success. Metadata written only after successful rename
- Continue with remaining skills when one fails (warn and skip) — matches existing main.rs pattern
- Auto-cleanup stale temp dirs (`.skills-sync-tmp-*`) at start of each sync run
- Non-zero exit code after all syncs complete if any skill failed — report both successes and failures in summary

### Claude's Discretion
- Exact temp directory naming scheme
- Git CLI flags for shallow clone and sparse checkout
- Whether to use `git` CLI or `git2` crate
- Internal error type additions for sync-specific failures

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SyncAction` enum (Install/Update/Remove/Skip): Drives the sync engine — each variant contains repo, subdirectory, and SHA info
- `SkillMeta` + `write_metadata_atomic()`: Atomic metadata writes via tempfile+rename already implemented
- `read_metadata()`: Reads existing metadata for SHA comparison
- `GitHubClient`: Auth discovery and API calls already handled; sync engine consumes resolver output, not API directly
- `load_local_metadata()`: Scans `.claude/skills/` for metadata files matching a repo

### Established Patterns
- Error handling: `miette` diagnostics with `SkillsSyncError` enum — sync errors should follow same pattern
- Warn-and-skip: Current main.rs warns on inaccessible repos and continues — orphan/failure handling follows same pattern
- Atomic writes: tempfile crate already in use for metadata; extend pattern to directory installs

### Integration Points
- `main.rs:run()` currently stops after printing sync plan summary — sync engine executes the `all_actions` vector
- `config.rs`: Manifest discovery already supports local vs global paths — sync engine needs the corresponding target directory
- `cli.rs`: `--global` flag already parsed — sync engine reads this to determine target directory

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-sync-engine*
*Context gathered: 2026-03-02*
