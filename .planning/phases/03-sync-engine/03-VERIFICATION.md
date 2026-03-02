---
phase: 03-sync-engine
verified: 2026-03-02T15:00:00Z
status: human_needed
score: 16/16 must-haves verified
re_verification: true
  previous_status: human_needed
  previous_score: 9/9
  gaps_closed:
    - "Plans 04 and 05 executed after previous verification: manifest subdir format and sibling lock file placement now verified"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Install a real skill from a single-skill GitHub repo"
    expected: "Skill directory appears at .claude/skills/owner--repo/ with sibling lock file .claude/skills/owner--repo.lock containing source, sha, and timestamps"
    why_human: "execute_install requires real git binary and network access; unit tests cover only helper functions and execute_remove"
  - test: "Install a monorepo skill from a GitHub repo with SKILL.md in a subdirectory"
    expected: "Only the subdirectory contents appear at .claude/skills/owner--repo--subdir/, not the full repo; sibling lock file at .claude/skills/owner--repo--subdir.lock"
    why_human: "4-step sparse checkout path requires real git and network; not covered by automated tests"
  - test: "Install using manifest entry owner/repo/path/to/skill (explicit subdir format)"
    expected: "Tree API detection is skipped; skill installed using specified subdir directly; dir name uses -- separator flattening"
    why_human: "Requires real git and network to confirm end-to-end integration of parse_repo_entry through execute_install"
  - test: "Interrupt an install mid-clone (kill process), then re-run"
    expected: "No corrupted directory at final path; re-run completes cleanly (stale temp dir cleaned by cleanup_stale_temp_dirs)"
    why_human: "Temp-dir-then-rename atomicity guarantee requires real filesystem interaction and process interruption"
  - test: "Run sync with an orphaned skill directory (source not in manifest) in .claude/skills/"
    expected: "Warning lists the orphan, Y/n prompt appears, confirmation deletes both the directory and its sibling .lock file"
    why_human: "dialoguer TTY interaction cannot be tested programmatically; requires interactive terminal"
  - test: "Run skills-sync --global and confirm target is ~/.claude/skills/"
    expected: "Skill directory appears at ~/.claude/skills/owner--repo/ with sibling lock at ~/.claude/skills/owner--repo.lock rather than ./.claude/skills/"
    why_human: "Requires real git and network; programmatic verification confirms code path but not directory placement"
---

# Phase 3: Sync Engine Verification Report

**Phase Goal:** The tool can install, update, and remove skills on disk, targeting local or global directories, while leaving no corrupted state on network failure or interruption
**Verified:** 2026-03-02T15:00:00Z
**Status:** human_needed (all 16 automated checks pass; 6 items require human verification)
**Re-verification:** Yes — supersedes previous verification (plans 01-03); now includes plans 04 and 05 (manifest subdir format and sibling lock file placement)

## Goal Achievement

### Observable Truths

| #  | Truth                                                                                           | Status     | Evidence                                                                                                     |
|----|-------------------------------------------------------------------------------------------------|------------|--------------------------------------------------------------------------------------------------------------|
| 1  | Single-skill install clones repo with --depth=1 to temp dir, renames atomically                | VERIFIED   | sync.rs L111-131: run_git(clone --depth=1), fs::rename to final_path                                        |
| 2  | Monorepo install uses 4-step sparse checkout, extracts subdirectory only                        | VERIFIED   | sync.rs L135-169: clone --no-checkout --filter=blob:none, sparse-checkout init, set, checkout, rename subdir |
| 3  | Update preserves installed_at, removes old dir, re-installs, patches timestamp                  | VERIFIED   | sync.rs L202-228: read_metadata(sibling lock) -> installed_at, remove_dir_all, execute_install, patch back  |
| 4  | Metadata lock file written ONLY after successful atomic rename, as sibling to skill dir         | VERIFIED   | sync.rs L179-185: meta_path_for_skill(target_dir, &dir_name), write_metadata_atomic after git_result check  |
| 5  | Temp dir (.skills-sync-tmp-{pid}-{name}) inside target_dir ensures same-filesystem atomic rename| VERIFIED   | sync.rs L100-101: format!(".skills-sync-tmp-{}-{}", std::process::id(), &dir_name)                          |
| 6  | Git failure during install leaves no final path; temp dir cleaned up                           | VERIFIED   | sync.rs L173-177: on Err, remove_dir_all(&tmp_path) before returning error                                  |
| 7  | Stale temp dirs cleaned at start of each sync run                                               | VERIFIED   | sync.rs L254-267: cleanup_stale_temp_dirs scans for .skills-sync-tmp-* entries                              |
| 8  | Orphaned skills detected and batched into single Y/n prompt                                     | VERIFIED   | sync.rs L412-466: scan_orphans + prompt_orphan_removal, dialoguer::Confirm::new().interact_opt()            |
| 9  | Sync target defaults to .claude/skills/ (local); global target via --global                    | VERIFIED   | cli.rs L19-21: pub global: bool with #[arg(long)]; main.rs L63-72: conditional dirs::home_dir() resolution  |
| 10 | Manifest entry owner/repo/subdir parsed — first two segments as owner/repo, rest as subdir     | VERIFIED   | manifest.rs L32-38: parse_repo_entry via splitn(3, '/'); regex updated to accept /segment suffixes          |
| 11 | When manifest specifies subdir, Trees API detection is skipped; specified subdir used directly  | VERIFIED   | main.rs L131-145: if let Some(subdir) = manifest_subdir branch creates RemoteSkill without detect_skills    |
| 12 | skill_dir_name flattens multi-segment subdir slashes to -- separator                            | VERIFIED   | sync.rs L23: subdir.replace('/', "--"); test skill_dir_name_multi_segment_subdir passes                     |
| 13 | Orphan detection uses owner/repo (not full manifest path) in manifest_repos                     | VERIFIED   | main.rs L87-88: repo_slug = format!("{}/{}", owner, repo_name); manifest_repos.insert(repo_slug)            |
| 14 | Lock files stored as siblings to skill dirs at target_dir level (owner--repo.lock)             | VERIFIED   | metadata.rs L18-20: meta_path_for_skill returns target_dir.join(format!("{}.{}", dir_name, "lock"))        |
| 15 | Orphan removal deletes both skill directory and its sibling lock file                           | VERIFIED   | sync.rs L454-455: meta_path_for_skill(target_dir, &orphan.dir_name), execute_remove(dir, Some(&lock_path)) |
| 16 | load_local_metadata reads sibling *.lock files at target_dir level (not inside subdirs)        | VERIFIED   | main.rs L246-263: scans entries, checks is_file() + ext == META_EXTENSION, reads metadata from path         |

**Score:** 16/16 truths verified

### Required Artifacts

| Artifact                  | Expected                                                             | Status   | Details                                                                                                                  |
|---------------------------|----------------------------------------------------------------------|----------|--------------------------------------------------------------------------------------------------------------------------|
| `src/sync.rs`             | Sync engine: install, update, remove, orphan detection, orchestration | VERIFIED | 485 lines; skill_dir_name, execute_install, execute_update, execute_remove, cleanup_stale_temp_dirs, scan_orphans, execute_sync_plan all present and substantive |
| `src/error.rs`            | GitFailed, GitNotFound, RenameFailed, RemoveFailed, SyncFailed       | VERIFIED | All 5 variants present (L74-106); use #[source] not #[from] to avoid conflicting From impls                             |
| `src/lib.rs`              | pub mod sync; declaration                                            | VERIFIED | pub mod sync; present (L9)                                                                                               |
| `Cargo.toml`              | dialoguer = "0.12" dependency                                        | VERIFIED | dialoguer = "0.12" present (L18)                                                                                         |
| `tests/sync_tests.rs`     | Unit tests for naming, URL building, fs operations, sibling lock files | VERIFIED | 111 lines; 11 tests pass: skill_dir_name (4), build_clone_url (2), meta_path (2), execute_remove (3)                   |
| `src/main.rs`             | Full pipeline: parse_repo_entry, branch_map, execute_sync_plan       | VERIFIED | parse_repo_entry called at L83; branch_map + manifest_repos built in loop; execute_sync_plan called at L209             |
| `src/cli.rs`              | --global flag                                                        | VERIFIED | pub global: bool field with #[arg(long)] and doc comment at L19-21                                                      |
| `src/manifest.rs`         | Updated regex, parse_repo_entry helper for owner/repo/subdir format  | VERIFIED | REPO_PATTERN at L9 accepts /segment suffixes; parse_repo_entry exported at L32                                          |
| `src/metadata.rs`         | meta_path_for_skill helper and META_EXTENSION constant               | VERIFIED | META_EXTENSION at L11; meta_path_for_skill at L18; META_FILENAME retained for backward compat                           |
| `src/init.rs`             | Updated template documenting owner/repo/subdir format                | VERIFIED | init_tests passes test_init_content_has_example_repos and test_init_generated_manifest_passes_validation                |
| `tests/manifest_tests.rs` | Tests for monorepo manifest entries with subdirectory paths          | VERIFIED | 27 tests pass including test_valid_monorepo_subdir, test_parse_repo_entry_with_subdir, test_duplicate_different_subdirs  |

### Key Link Verification

| From              | To                      | Via                                                                          | Status   | Details                                                                                               |
|-------------------|-------------------------|------------------------------------------------------------------------------|----------|-------------------------------------------------------------------------------------------------------|
| `src/sync.rs`     | `src/error.rs`          | SkillsSyncError variants for git/fs failures                                 | VERIFIED | sync.rs L5: use SkillsSyncError; GitFailed, RenameFailed, RemoveFailed, SyncFailed used throughout    |
| `src/sync.rs`     | `src/metadata.rs`       | write_metadata_atomic, meta_path_for_skill, read_metadata, SkillMeta         | VERIFIED | sync.rs L6: use write_metadata_atomic, SkillMeta; L180, L206, L222, L304, L454: meta_path_for_skill  |
| `src/sync.rs`     | `std::process::Command` | git subprocess invocation                                                    | VERIFIED | sync.rs L3: use std::process::Command; L49: Command::new("git")                                      |
| `src/main.rs`     | `src/sync.rs`           | execute_sync_plan call                                                       | VERIFIED | main.rs L10: use skills_sync::sync; L209: sync::execute_sync_plan(...)                               |
| `src/main.rs`     | `src/sync.rs`           | cleanup_stale_temp_dirs called inside execute_sync_plan before installs      | VERIFIED | sync.rs L354: cleanup_stale_temp_dirs(target_dir) first line of execute_sync_plan                    |
| `src/sync.rs`     | `dialoguer::Confirm`    | Batch orphan removal confirmation                                            | VERIFIED | sync.rs L330-337: dialoguer::Confirm::new().with_prompt(...).interact_opt()                          |
| `src/sync.rs`     | `src/metadata.rs`       | read_metadata via meta_path_for_skill during orphan scan                     | VERIFIED | sync.rs L304: meta_path_for_skill(target_dir, &name_str); L305: read_metadata(&meta_path)           |
| `src/main.rs`     | `src/cli.rs`            | cli.global gates target_dir path selection                                   | VERIFIED | main.rs L64: if cli.global { ... } else { PathBuf::from(".claude/skills") }                         |
| `src/main.rs`     | `dirs::home_dir()`      | Resolves ~ to absolute path when --global is set                             | VERIFIED | main.rs L65: dirs::home_dir().ok_or_else(...) with SyncFailed error on None                         |
| `src/main.rs`     | `src/manifest.rs`       | parse_repo_entry extracts (owner, repo_name, subdir) from manifest entry     | VERIFIED | main.rs L7: use skills_sync::manifest::parse_repo_entry; L83: parse_repo_entry(repo_entry)          |
| `src/main.rs`     | `src/resolver.rs`       | When subdir present, creates RemoteSkill directly instead of detect_skills   | VERIFIED | main.rs L131-145: if let Some(subdir) = manifest_subdir branch bypasses detect_skills               |
| `src/sync.rs`     | `src/sync.rs`           | skill_dir_name replaces slashes in multi-segment subdirs with --             | VERIFIED | sync.rs L23: subdir.replace('/', "--")                                                              |
| `src/main.rs`     | `src/metadata.rs`       | load_local_metadata uses META_EXTENSION to scan sibling *.lock files         | VERIFIED | main.rs L252: ext == metadata::META_EXTENSION; reads from path (sibling, not inside subdir)         |
| `src/sync.rs`     | `src/sync.rs`           | execute_remove deletes both skill dir and sibling lock file                   | VERIFIED | sync.rs L236-245: remove_dir_all(skill_path) then remove_file(lp) best-effort                       |

### Requirements Coverage

| Requirement | Source Plans | Description                                                         | Status    | Evidence                                                                                        |
|-------------|--------------|---------------------------------------------------------------------|-----------|-------------------------------------------------------------------------------------------------|
| SYNC-01     | 03-01, 03-04 | Install missing skills via shallow git clone (single-skill)         | SATISFIED | execute_install single-skill path: run_git(clone --depth=1, -b branch); parse_repo_entry wires owner/repo extraction |
| SYNC-02     | 03-01, 03-04 | Install monorepo skills via sparse checkout                         | SATISFIED | execute_install monorepo path: 4-step sparse checkout; manifest owner/repo/subdir format bypasses detect_skills |
| SYNC-04     | 03-02, 03-05 | Detect orphaned skills and prompt for removal                       | SATISFIED | scan_orphans reads sibling lock files; prompt_orphan_removal + dialoguer Y/n; execute_remove cleans both dir and lock |
| SYNC-06     | 03-02, 03-03 | Sync to local (.claude/skills/) by default or global with --global  | SATISFIED | Local default: PathBuf::from(".claude/skills"); global: dirs::home_dir().join(".claude/skills") gated on cli.global |
| ERR-03      | 03-02        | Inaccessible repos warned and skipped; remaining continue           | SATISFIED | main.rs L93-103: warn-and-continue on API failure; sync.rs: per-skill warn-and-skip in execute_sync_plan |
| ERR-04      | 03-01, 03-05 | Network failures mid-sync leave no corrupted state                  | SATISFIED | Temp-dir-then-rename pattern; metadata (sibling lock) written only after successful rename     |

**Orphaned requirements check:** REQUIREMENTS.md traceability table maps SYNC-01, SYNC-02, SYNC-04, SYNC-06, ERR-03, ERR-04 to Phase 3. All 6 accounted for across plans 03-01 through 03-05. No orphaned requirements.

All 6 requirements marked as Complete in REQUIREMENTS.md.

### Anti-Patterns Found

| File          | Line | Pattern                                | Severity | Impact                                                         |
|---------------|------|----------------------------------------|----------|----------------------------------------------------------------|
| src/sync.rs   | 337  | `_ => false` matches all non-TTY cases | Info     | Correct behavior for non-interactive terminals; not a stub    |
| src/metadata.rs | 8  | META_FILENAME const retained           | Info     | Labeled "legacy, kept for backward compatibility" — not used in sync or main; harmless dead constant |

No blockers. No placeholder implementations. No TODO/FIXME comments in any modified file. All 94 tests pass (11 sync, 27 manifest, 14 metadata, 18 resolver, 8 init, 10 github, 6 config).

### Human Verification Required

#### 1. Single-Skill Install End-to-End

**Test:** Run `cargo run` against a manifest containing a single-skill GitHub repo (e.g., `repos = ["owner/repo"]` where SKILL.md is at the repo root).
**Expected:** Skill directory appears at `.claude/skills/owner--repo/` containing the cloned files, with a sibling lock file at `.claude/skills/owner--repo.lock` containing correct `source`, `sha`, and timestamps. No `.skills.lock` file inside the skill directory.
**Why human:** `execute_install` requires a real git binary and network access. Unit tests cover only `skill_dir_name`, `build_clone_url`, `meta_path_for_skill`, and `execute_remove` (filesystem only).

#### 2. Monorepo Sparse Checkout with Explicit Subdir

**Test:** Run sync against a manifest with an explicit subdir entry such as `repos = ["anthropics/skills/skills/pdf"]`.
**Expected:** Only the `skills/pdf` subdirectory contents appear at `.claude/skills/anthropics--skills--skills--pdf/` (flat name, no nested slash). Sibling lock at `.claude/skills/anthropics--skills--skills--pdf.lock`. Trees API detection is NOT called — the specified subdir is used directly.
**Why human:** 4-step sparse checkout + parse_repo_entry integration requires real git and network. Multi-segment slash flattening is confirmed by unit test but the full path requires runtime observation.

#### 3. Atomic Install Guarantee Under Interruption

**Test:** Start a sync that installs a skill, then interrupt the process (Ctrl-C or kill) mid-clone before the rename completes. Re-run the tool.
**Expected:** No directory exists at the final path after interruption. No sibling lock file written. On re-run, the install completes cleanly (stale temp dir cleaned by `cleanup_stale_temp_dirs` at the start of the new run).
**Why human:** Requires real process interruption and filesystem observation. The temp-dir-then-rename guarantee cannot be tested programmatically without running actual git operations.

#### 4. Orphan Detection, Prompt, and Sibling Lock File Removal

**Test:** Install a skill, then remove its repo from the manifest, then run sync in an interactive terminal.
**Expected:** Warning lists the orphan by directory name and source. `Remove orphaned skills? [y/N]` prompt appears. Answering `y` removes both the skill directory AND its sibling `.lock` file. Answering `n` or pressing Enter leaves both intact. Non-TTY (e.g., piped via `echo "" |`) silently skips removal.
**Why human:** `dialoguer::Confirm::new().interact_opt()` requires an interactive TTY. The lock file removal is wired via `execute_remove(&orphan.dir_path, Some(&lock_path))` in execute_sync_plan, but the combined behavior requires runtime observation.

#### 5. Global Target Routing via --global Flag

**Test:** Run `cargo run -- --global` against a manifest with a known skill repo.
**Expected:** Skill directory appears at `~/.claude/skills/owner--repo/` with sibling lock at `~/.claude/skills/owner--repo.lock`, rather than `./.claude/skills/`. Run without `--global` to confirm it returns to the local path.
**Why human:** Requires real git and network to trigger execute_install. Code path is confirmed correct (`cli.global` conditional, `dirs::home_dir()` call verified), but actual directory placement under `~` requires runtime observation.

#### 6. load_local_metadata Reads Sibling Lock Files for Update Detection

**Test:** Install a skill (creates sibling `.lock` file), confirm SHA is stored, change the remote repo, re-run sync.
**Expected:** `load_local_metadata` detects the existing sibling lock, resolver sees the old SHA differs from remote, produces an Update action. The update preserves the original `installed_at` timestamp.
**Why human:** Requires real git, network, and SHA comparison against actual remote HEAD.

### Gaps Summary

No gaps remaining. All 16 truths verified across all five plans. The 16 truths include 9 from the previous verification (plans 01-03) plus 7 new truths from plans 04 and 05:

**Plans 04 and 05 additions (verified clean):**
1. Manifest entry `owner/repo/subdir` format accepted — regex updated, `parse_repo_entry` added (manifest.rs L9, L32)
2. Trees API detection bypassed when manifest specifies subdir (main.rs L131-145)
3. `skill_dir_name` flattens multi-segment subdir slashes (sync.rs L23)
4. `manifest_repos` uses `owner/repo` slug (not full entry) for orphan detection consistency (main.rs L87-88)
5. Metadata lock files moved to sibling position at target_dir level: `meta_path_for_skill` helper added (metadata.rs L18)
6. `execute_remove` accepts `Option<&Path>` lock_path and deletes sibling lock file (sync.rs L236)
7. `load_local_metadata` scans `*.lock` files at target_dir level using `META_EXTENSION` (main.rs L246-263)

All 94 tests pass. Clean build. No regressions introduced by plans 04 or 05 against plans 01-03 behavior.

---

_Verified: 2026-03-02T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification after plans 04 and 05 execution (manifest subdir format + sibling lock file placement)_
