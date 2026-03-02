---
status: complete
phase: 03-sync-engine
source: 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md
started: 2026-03-02T11:00:00Z
updated: 2026-03-02T11:05:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Install a skill from GitHub
expected: Running skills-sync with a manifest containing a new skill clones it into the target skills directory. The skill directory is named owner--repo. A .metadata.json file is created inside the skill directory after successful install.
result: pass

### 2. Monorepo skill sparse checkout
expected: A skill pointing to a subdirectory in a monorepo (owner/repo/subdir) installs only that subdirectory. The skill directory is named owner--repo--subdir. Only files from the specified subdirectory are present.
result: issue
reported: "invalid repo format: `anthropics/skills/skills/skill-creator` - Repos must match `owner/repo` format (e.g., `anthropics/claude-skills`)"
severity: blocker

### 3. Update an existing skill
expected: Running sync again when a skill is already installed updates it to the latest version. The installed_at timestamp in .metadata.json is preserved from the original install (not overwritten).
result: issue
reported: "this is no metadata.json. only a skills.lock file but way down in the actual cloned skill instead of the project directory or global directory"
severity: major

### 4. End-to-end sync pipeline
expected: Running skills-sync with a manifest file containing multiple skills installs new ones, updates existing ones, and reports results. Skills that haven't changed show as skipped.
result: skipped
reason: Blocked by monorepo/subdir parsing issue (test 2) — almost all repos contain skills in a subdirectory

### 5. Orphan detection and batch removal
expected: Skills installed in the skills directory but not listed in the manifest are detected as orphans. A single Y/n confirmation prompt lists all orphans and asks to remove them in one batch (not per-skill prompts).
result: [pending]

### 6. Stale temp directory cleanup
expected: Any leftover .skills-sync-tmp-* directories in the target directory (from previously interrupted syncs) are automatically removed at the start of each sync run.
result: [pending]

### 7. Per-skill failure resilience
expected: If one skill fails to sync (e.g., bad repo URL), remaining skills still sync successfully. A warning is printed for the failed skill and the process exits with non-zero exit code.
result: [pending]

### 8. --global flag
expected: Running with --global installs skills to ~/.claude/skills/ instead of the local .claude/skills/ directory. Without --global, the default local directory is used.
result: [pending]

## Summary

total: 8
passed: 1
issues: 2
pending: 4
skipped: 1

## Gaps

- truth: "Monorepo skill with owner/repo/subdir format installs only the subdirectory"
  status: failed
  reason: "User reported: invalid repo format: `anthropics/skills/skills/skill-creator` - Repos must match `owner/repo` format"
  severity: blocker
  test: 2
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Metadata file (.metadata.json) created in skills directory after install, preserving installed_at on updates"
  status: failed
  reason: "User reported: this is no metadata.json. only a skills.lock file but way down in the actual cloned skill instead of the project directory or global directory"
  severity: major
  test: 3
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
