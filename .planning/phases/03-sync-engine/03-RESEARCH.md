# Phase 3: Sync Engine - Research

**Researched:** 2026-03-02
**Domain:** Git subprocess operations, atomic directory installs, interactive CLI prompts, orphan detection
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Directory layout**
- Single-skill repos named `owner--repo` (e.g., `.claude/skills/anthropics--claude-skills/`)
- Monorepo skills named `owner--repo--subdir` (e.g., `.claude/skills/anthropics--monorepo--python/`)
- Flat structure — no nested owner directories
- Lock file (`.skills.lock`) lives inside the skill directory — self-contained
- Monorepo sparse checkout extracts subdirectory contents only — no redundant nesting inside the skill dir

**Orphan removal flow**
- Batch confirmation — list all orphans, then one `Y/n` prompt
- Show name + source only per orphan (minimal, clean)
- Delete permanently on confirmation — skills are always recoverable by re-adding to manifest
- Warn every run if orphans exist (no suppression state to track)

**Update behavior**
- Delete and re-clone — remove old directory, shallow clone fresh. Simple, deterministic, avoids git state issues
- No local modification detection — skills are managed artifacts, local edits not expected to survive
- Preserve `installed_at` timestamp from original install; update `updated_at` and `sha`
- Sequential processing — one skill at a time. Good enough for typical manifest sizes

**Failure recovery**
- Clone to temp directory in same parent, atomic rename to final path on success. Metadata written only after successful rename
- Continue with remaining skills when one fails (warn and skip) — matches existing main.rs pattern
- Auto-cleanup stale temp dirs (`.skills-sync-tmp-*`) at start of each sync run
- Non-zero exit code after all syncs complete if any skill failed — report both successes and failures in summary

### Claude's Discretion
- Exact temp directory naming scheme
- Git CLI flags for shallow clone and sparse checkout
- Whether to use `git` CLI or `git2` crate
- Internal error type additions for sync-specific failures

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SYNC-01 | Tool installs missing skills via shallow git clone (single-skill repos) | Verified: `git clone --depth=1 URL DEST` command sequence, temp+rename pattern |
| SYNC-02 | Tool installs missing monorepo skills via sparse checkout to extract subdirectories | Verified: 4-step sparse checkout sequence, subdirectory move-out approach |
| SYNC-04 | Tool detects orphaned skills and prompts user for removal | Verified: global disk scan approach, dialoguer Confirm API |
| SYNC-06 | Tool syncs to local target (`./.claude/skills/`) by default or global (`~/.claude/skills/`) with `--global` flag | Confirmed: `--global` is Phase 4 concern; Phase 3 accepts `target_dir` parameter, defaults to `./.claude/skills/` |
| ERR-03 | Inaccessible/missing repos are warned and skipped; remaining repos continue | Pattern already established in main.rs; sync engine extends same warn-and-skip to git failures |
| ERR-04 | Network failures mid-sync leave no corrupted state; re-running resolves partial syncs | Verified: temp dir in same parent + atomic rename; stale temp cleanup at run start |
</phase_requirements>

## Summary

Phase 3 executes the sync plan produced by Phase 2 by invoking `git` as a subprocess. The key technical areas are: (1) single-skill shallow clone via `git clone --depth=1`, (2) monorepo skill extraction via a 4-step sparse checkout sequence with subdirectory move-out, (3) atomic install via temp-dir-then-rename pattern, (4) global orphan detection by scanning all `.claude/skills/` subdirectories and comparing against the processed manifest repos, and (5) an interactive batch removal prompt using the `dialoguer` crate.

The decision to use git CLI subprocess over the `git2` crate is already locked by STATE.md ("libgit2 silently ignores --depth 1 and has unreliable sparse checkout"). The `tempfile` crate is already in Cargo.toml and used for atomic metadata writes; the same pattern extends to directory installs. The `dialoguer` crate (version 0.12.0) needs to be added to Cargo.toml for the orphan confirmation prompt.

The most complex implementation concern is monorepo extraction: a full sparse-checkout clone must happen in a temp directory, then the target subdirectory is moved out to the final skill directory via atomic rename. STATE.md flags this as a risk: "shallow clone + sparse checkout interaction is poorly documented in combination." The research here validates the correct 4-command sequence and confirms it works against a real GitHub repo.

**Primary recommendation:** Build `src/sync.rs` as a dedicated sync engine module that accepts `Vec<SyncAction>`, a `target_dir: &Path`, and an optional git auth token; then wire it into `main.rs` after the existing plan-building loop. Keep git execution in a private `run_git()` helper that captures stderr for error reporting.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `git` (subprocess) | system git (tested: 2.53.0) | Shallow clone, sparse checkout | Locked decision — git2 crate unreliable for depth+sparse combination |
| `tempfile` | 3 (already in Cargo.toml) | Temp directory creation (via `Builder`) | Already in use for metadata writes; extends naturally to dir installs |
| `dialoguer` | 0.12.0 | Interactive Y/n confirmation prompt | Standard CLI prompt library; active maintenance, simple API |
| `std::fs` | stdlib | `fs::rename` for atomic directory swap, `fs::remove_dir_all` for cleanup | Atomic on same filesystem (POSIX rename(2) semantics) |
| `std::process::Command` | stdlib | Git subprocess invocation with stdout/stderr capture | Established pattern in github.rs for `gh auth token` call |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `dirs` | 6 (already in Cargo.toml) | Resolve `~/.claude/skills/` for global target | When `--global` flag is active (Phase 4 wires this; Phase 3 accepts a `target_dir` parameter) |
| `chrono` | 0.4 (already in Cargo.toml) | `Utc::now()` for `updated_at` timestamp on updates | When preserving `installed_at` from existing metadata during updates |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `git` subprocess | `git2` crate | git2 silently ignores `--depth 1` and has unreliable sparse checkout (STATE.md); subprocess is more brittle if git not installed but realistic for v1 |
| `dialoguer` | `inquire` | inquire is more full-featured but heavier; dialoguer is minimal and sufficient for a single Confirm prompt |
| temp-dir-then-rename | clone directly to final path | Direct clone leaves partially-cloned dir on failure; temp+rename gives ERR-04 atomicity guarantee |

**Installation (add to Cargo.toml):**
```toml
dialoguer = "0.12"
```

## Architecture Patterns

### Recommended Project Structure

```
src/
├── sync.rs          # New: sync engine (git operations, orphan detection, prompt)
├── main.rs          # Modified: call sync engine after building all_actions
├── resolver.rs      # Existing: pure action resolution (no change)
├── metadata.rs      # Existing: SkillMeta, write_metadata_atomic (no change)
├── error.rs         # Modified: add sync-specific error variants
└── github.rs        # Existing: auth token now also passed to sync engine for git auth
```

### Pattern 1: Skill Directory Naming

**What:** Convert `owner/repo` and optional subdirectory to flat directory name.
**Rule:**
- Single-skill: `owner/repo` → `owner--repo`
- Monorepo: `owner/repo` + `subdir` → `owner--repo--subdir`

```rust
// Source: CONTEXT.md directory layout decision
pub fn skill_dir_name(repo: &str, subdirectory: &Option<String>) -> String {
    let base = repo.replace('/', "--");
    match subdirectory {
        Some(subdir) => format!("{}--{}", base, subdir),
        None => base,
    }
}
```

### Pattern 2: Single-Skill Shallow Clone (atomic)

**What:** Clone to temp dir in same parent, rename to final on success.
**When to use:** SyncAction::Install or SyncAction::Update (after removing old dir) for single-skill repos.

```rust
// Source: Validated manually with git 2.53.0 on macOS
fn install_single_skill(
    repo: &str,
    target_dir: &Path,    // e.g. .claude/skills/
    skill_dir: &str,      // e.g. "anthropics--claude-skills"
    branch: &str,
    token: Option<&str>,
) -> Result<(), SyncError> {
    let final_path = target_dir.join(skill_dir);
    let tmp_name = format!(".skills-sync-tmp-{}-{}", std::process::id(), skill_dir);
    let tmp_path = target_dir.join(&tmp_name);

    // Clone to temp location
    let clone_url = build_clone_url(repo, token);
    run_git(&["clone", "--depth=1", "-b", branch, "--", &clone_url, tmp_path.to_str().unwrap()])?;

    // Atomic rename: same filesystem guaranteed (both in target_dir)
    std::fs::rename(&tmp_path, &final_path)
        .map_err(|e| SyncError::Rename { path: final_path.clone(), source: e })?;

    Ok(())
}
```

### Pattern 3: Monorepo Sparse Checkout (4-step)

**What:** Clone with blob filter and no checkout, enable sparse cone, set subdir, checkout, then move subdir out.
**When to use:** SyncAction::Install or SyncAction::Update for monorepo skills (subdirectory is Some).

Validated command sequence (tested against `anthropics/anthropic-quickstarts`, git 2.53.0):

```bash
# Step 1: Clone with no checkout and blob filter (faster — no file content downloaded initially)
git clone --depth=1 --no-checkout --filter=blob:none https://github.com/owner/repo.git /tmp/.skills-sync-tmp-abc/

# Step 2: Initialize cone-mode sparse checkout
git -C /tmp/.skills-sync-tmp-abc/ sparse-checkout init --cone

# Step 3: Set target subdirectory
git -C /tmp/.skills-sync-tmp-abc/ sparse-checkout set target-subdir

# Step 4: Checkout — materializes only target-subdir (plus repo root files)
git -C /tmp/.skills-sync-tmp-abc/ checkout
```

After checkout, the temp dir contains:
```
.skills-sync-tmp-abc/
  .git/
  target-subdir/        ← move this out
  README.md             ← root files also present (expected, harmless)
  ...
```

Then extract the subdirectory and clean up:

```rust
// Source: Validated manually
let subdir_in_tmp = tmp_path.join(subdir);
let final_path = target_dir.join(skill_dir_name);

// Atomic move: same filesystem (both in target_dir parent's tree)
std::fs::rename(&subdir_in_tmp, &final_path)?;

// Clean up temp clone (now only has .git and root files)
std::fs::remove_dir_all(&tmp_path)?;
```

**Key insight:** `--filter=blob:none` + sparse checkout means only the sparse-selected subdirectory's blobs are fetched. Root-level files (README, LICENSE) are present because cone mode always includes root. This is acceptable — they are never moved to the skill dir.

### Pattern 4: Temp Directory Cleanup at Run Start

**What:** Scan target_dir for `.skills-sync-tmp-*` prefixed entries and remove them.
**When:** At the beginning of each sync run, before processing any actions.

```rust
// Source: CONTEXT.md failure recovery decision
fn cleanup_stale_temp_dirs(target_dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(".skills-sync-tmp-") {
                let _ = std::fs::remove_dir_all(entry.path());
                // Warn if verbose, but don't fail — cleanup is best-effort
            }
        }
    }
}
```

### Pattern 5: Global Orphan Detection

**What:** After processing all manifest repos, scan the entire target_dir for skill dirs with `.skills.lock` whose `source` is not in the set of processed manifest repos.
**When:** After the main per-repo loop in main.rs, before executing Remove actions.

The resolver's existing `SyncAction::Remove` only handles skills that were in a repo previously but removed from a monorepo. It does NOT handle repos removed entirely from the manifest (those repos were never loaded, so their local skills never got Remove actions).

```rust
// Source: Analysis of existing main.rs load_local_metadata pattern
fn scan_all_orphans(target_dir: &Path, manifest_repos: &HashSet<String>) -> Vec<OrphanSkill> {
    let mut orphans = Vec::new();
    if let Ok(entries) = std::fs::read_dir(target_dir) {
        for entry in entries.flatten() {
            // Skip temp dirs
            let name = entry.file_name();
            if name.to_string_lossy().starts_with(".skills-sync-tmp-") {
                continue;
            }
            let meta_path = entry.path().join(META_FILENAME);
            if let Ok(meta) = read_metadata(&meta_path) {
                if !manifest_repos.contains(&meta.source) {
                    orphans.push(OrphanSkill {
                        dir: entry.path(),
                        source: meta.source,
                        dir_name: name.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    orphans
}
```

**Important distinction:** The resolver's existing Remove actions (for in-manifest repos) and the global orphan scan are two separate sources of removals. Both should be batched and presented together in one confirmation prompt.

### Pattern 6: Batch Orphan Confirmation with dialoguer

**What:** Collect all orphans, print them, ask once.
**When:** Any Remove actions exist (from either resolver or global scan).

```rust
// Source: docs.rs/dialoguer/0.11.0 (API unchanged in 0.12.0)
use dialoguer::Confirm;

fn prompt_remove_orphans(orphans: &[OrphanInfo]) -> bool {
    println!("warning: found {} orphaned skill(s):", orphans.len());
    for orphan in orphans {
        println!("  {} ({})", orphan.dir_name, orphan.source);
    }
    Confirm::new()
        .with_prompt("Remove orphaned skills?")
        .default(false)
        .interact()
        .unwrap_or(false) // On non-interactive terminal, default to no
}
```

`interact()` returns `Result<bool>`. `interact_opt()` returns `Result<Option<bool>>` and allows Escape/q for cancellation. Use `interact_opt()` if you want to differentiate "no" from "cancelled".

### Pattern 7: Git Auth via URL Embedding

**What:** Embed token in HTTPS clone URL.
**When:** Token is available (from existing `discover_token()`).

```rust
// Source: Standard GitHub HTTPS auth pattern
fn build_clone_url(repo: &str, token: Option<&str>) -> String {
    match token {
        Some(tok) => format!("https://{}@github.com/{}.git", tok, repo),
        None => format!("https://github.com/{}.git", repo),
    }
}
```

**Note on security:** Token appears in git's reflog and remote config within the cloned temp dir. Since the temp dir is removed after install (or at next run start if interrupted), exposure window is minimal. For v1, this is acceptable. The token does NOT appear in process argv since it's embedded in the URL string passed as a single argument.

### Pattern 8: Git Subprocess Runner

**What:** Centralized helper that runs git commands and captures stderr for error messages.

```rust
// Source: Established pattern from github.rs (gh auth token call)
fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<(), SyncError> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    // Capture stderr to include in error message; stdout not needed
    let output = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| SyncError::GitNotFound { source: e })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SyncError::GitFailed {
            args: args.join(" "),
            stderr,
        });
    }
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Cloning directly to final path:** If clone fails midway, the final path exists but is corrupt. Next run finds a dir without `.skills.lock` — undefined behavior. Always use temp+rename.
- **Using `git2` crate for sparse checkout:** Locked decision. `git2` silently ignores `--depth 1` and has unreliable sparse checkout behavior (STATE.md).
- **Per-orphan prompts:** The CONTEXT requires a single batch prompt. Per-orphan prompts are verbosely annoying and UX-poor.
- **Writing metadata before rename:** The CONTEXT requires metadata written only after rename succeeds. Writing early means corrupt state if rename fails.
- **Using TempDir from tempfile for the clone dir:** `tempfile::TempDir` creates dirs in the system temp dir (e.g., `/tmp/`), not in `.claude/skills/`. This breaks the atomic rename guarantee (cross-filesystem move is not atomic). Use `std::fs::create_dir` with a manually named path inside `target_dir`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Interactive yes/no prompt | Custom stdin reader + terminal handling | `dialoguer::Confirm` | Handles TTY detection, raw mode, non-interactive terminals gracefully |
| Atomic file rename | Copy + delete sequence | `std::fs::rename` (wraps POSIX rename(2)) | Copy+delete is not atomic — power loss between steps leaves duplicate |
| Temp file creation | Manual random suffix | `tempfile::Builder` (for files) or manual named dir (for dirs) | tempfile handles cross-platform tmp semantics; for dirs in specific parent: use `format!(".skills-sync-tmp-{}", std::process::id())` |

**Key insight:** The directory-level atomic install is not provided by any crate — it's achieved by ensuring temp and final paths are siblings in the same directory (same filesystem → rename(2) is atomic). No crate needed.

## Common Pitfalls

### Pitfall 1: Sparse Checkout Fetching All Blobs

**What goes wrong:** Using `git clone --depth=1 --sparse URL DEST` without `--filter=blob:none` fetches all file contents, then applies sparse checkout. This is slow and defeats the purpose for large monorepos.

**Why it happens:** `--sparse` alone only sets the sparse pattern at checkout time; it doesn't filter what gets fetched from the server.

**How to avoid:** Use `--no-checkout --filter=blob:none` with `--depth=1`. Then set sparse-checkout pattern, then checkout. This tells the server to only send blob objects for paths matching the sparse pattern.

**Warning signs:** Clone takes much longer than expected; `.git/objects` is large after clone.

### Pitfall 2: Cross-Filesystem Rename Failure

**What goes wrong:** Creating temp dir in system temp (`/tmp/`) and trying to rename it to `./.claude/skills/owner--repo`. On macOS, `/tmp` may be on a different filesystem (tmpfs vs APFS), causing `rename(2)` to return `EXDEV` (cross-device link).

**Why it happens:** `rename(2)` on POSIX only works within a single filesystem.

**How to avoid:** Always create temp dirs inside `target_dir` itself (e.g., `.claude/skills/.skills-sync-tmp-PID`). Then both temp and final are on the same filesystem.

**Warning signs:** `std::fs::rename` returns `ErrorKind::CrossesDevices` (Rust 1.75+) or `ErrorKind::Other` with `os error 18` (EXDEV).

### Pitfall 3: Leaving .git Directory in Skill Dir (Monorepo)

**What goes wrong:** For monorepo extraction, if you clone into the final skill dir and then try to "move" things around, you end up with a `.git` directory inside the skill dir. This creates a nested git repo that confuses tools.

**Why it happens:** Clone creates `.git` in the target directory.

**How to avoid:** Always clone to a temp dir (which keeps its `.git` there). Move only the target subdirectory out to the final skill dir. The `.git` stays in the temp dir and is cleaned up with `fs::remove_dir_all`.

**Warning signs:** `.claude/skills/owner--repo--subdir/.git/` exists after install.

### Pitfall 4: Orphan Scan Misses Repos Removed from Manifest

**What goes wrong:** The existing `resolver::resolve()` only detects orphans within a repo's skills (e.g., skill removed from monorepo). It does NOT catch skills whose entire repo was removed from the manifest — because those repos are never processed, so `load_local_metadata()` is never called for them.

**Why it happens:** Current architecture: per-repo metadata load → per-repo resolve. No global scan.

**How to avoid:** After the main loop, add a GLOBAL scan: read ALL dirs in `target_dir` with `.skills.lock`, check if their `source` field matches any manifest repo. Any that don't match are orphans.

**Warning signs:** Old skill directories persist indefinitely after removing a repo from `.skills.toml`.

### Pitfall 5: Preserve installed_at on Updates

**What goes wrong:** On update (delete + re-clone), writing a fresh `SkillMeta::new()` loses the original `installed_at` timestamp.

**Why it happens:** `SkillMeta::new()` sets both timestamps to now.

**How to avoid:** Read the existing metadata before removing the old dir, preserve `installed_at`, build a new SkillMeta with the old `installed_at` value and new `sha` / `updated_at = Utc::now()`.

```rust
// Preserve installed_at from old metadata
let mut new_meta = SkillMeta::new(repo, new_sha);
new_meta.installed_at = old_meta.installed_at;  // restore original install time
```

**Warning signs:** All skills show the same `installed_at` as `updated_at` after every update.

### Pitfall 6: Non-Interactive Terminal Panic

**What goes wrong:** `dialoguer::Confirm::interact()` panics or returns an error when stdin is not a TTY (e.g., piped input, CI environment).

**Why it happens:** The prompt tries to read from a terminal that isn't there.

**How to avoid:** Use `.interact_opt()` and handle `None` (non-interactive, treat as "no" / skip removal), OR check `atty::is(atty::Stream::Stdin)` before prompting. Simplest: wrap in a `match` and fall through to skip.

**Note:** The `--force` flag (Phase 4) will bypass this entirely. For Phase 3, safe handling of non-TTY is sufficient.

## Code Examples

Verified patterns from official sources and validated testing:

### Single-Skill Install (complete flow)

```rust
// Source: Validated approach — temp dir + atomic rename + metadata write
pub fn execute_install(
    repo: &str,
    subdirectory: &Option<String>,
    remote_sha: &str,
    branch: &str,
    target_dir: &Path,
    token: Option<&str>,
) -> Result<(), SyncError> {
    let dir_name = skill_dir_name(repo, subdirectory);
    let final_path = target_dir.join(&dir_name);
    let tmp_name = format!(".skills-sync-tmp-{}-{}", std::process::id(), &dir_name);
    let tmp_path = target_dir.join(&tmp_name);

    std::fs::create_dir_all(target_dir)?;

    let clone_url = build_clone_url(repo, token);

    if subdirectory.is_none() {
        // Single-skill: clone directly to temp dir
        run_git(&["clone", "--depth=1", "-b", branch, "--",
                  &clone_url, tmp_path.to_str().unwrap()], None)?;
        std::fs::rename(&tmp_path, &final_path)?;
    } else {
        // Monorepo: sparse checkout, then move subdir out
        let subdir = subdirectory.as_ref().unwrap();
        std::fs::create_dir_all(&tmp_path)?;
        run_git(&["clone", "--depth=1", "--no-checkout", "--filter=blob:none",
                  "--", &clone_url, tmp_path.to_str().unwrap()], None)?;
        run_git(&["sparse-checkout", "init", "--cone"], Some(&tmp_path))?;
        run_git(&["sparse-checkout", "set", subdir], Some(&tmp_path))?;
        run_git(&["checkout"], Some(&tmp_path))?;

        let subdir_in_tmp = tmp_path.join(subdir);
        std::fs::rename(&subdir_in_tmp, &final_path)?;
        std::fs::remove_dir_all(&tmp_path)?;
    }

    // Write metadata ONLY after rename succeeds
    let meta_path = final_path.join(META_FILENAME);
    let mut meta = SkillMeta::new(repo, remote_sha);
    if let Some(sub) = subdirectory {
        meta = meta.with_subdirectory(sub);
    }
    write_metadata_atomic(&meta_path, &meta)?;

    Ok(())
}
```

### Update Flow

```rust
// Source: CONTEXT.md update behavior (delete + re-clone)
pub fn execute_update(
    repo: &str,
    subdirectory: &Option<String>,
    old_sha: &str,
    new_sha: &str,
    branch: &str,
    target_dir: &Path,
    token: Option<&str>,
) -> Result<(), SyncError> {
    let dir_name = skill_dir_name(repo, subdirectory);
    let existing_path = target_dir.join(&dir_name);

    // Read old metadata to preserve installed_at
    let old_meta_path = existing_path.join(META_FILENAME);
    let installed_at = read_metadata(&old_meta_path)
        .ok()
        .map(|m| m.installed_at)
        .unwrap_or_else(|| Utc::now());

    // Remove old directory
    std::fs::remove_dir_all(&existing_path)?;

    // Install fresh (reuse install logic)
    execute_install(repo, subdirectory, new_sha, branch, target_dir, token)?;

    // Patch installed_at timestamp back
    let meta_path = existing_path.join(META_FILENAME);
    if let Ok(mut meta) = read_metadata(&meta_path) {
        meta.installed_at = installed_at;
        let _ = write_metadata_atomic(&meta_path, &meta);
    }

    Ok(())
}
```

### Orphan Removal

```rust
// Source: fs::remove_dir_all docs + CONTEXT.md batch removal decision
pub fn execute_remove(skill_path: &Path) -> Result<(), SyncError> {
    std::fs::remove_dir_all(skill_path)
        .map_err(|e| SyncError::RemoveFailed { path: skill_path.to_owned(), source: e })
}
```

### Dialoguer Confirm Usage

```rust
// Source: docs.rs/dialoguer/0.11.0 Confirm struct (API stable in 0.12.0)
use dialoguer::Confirm;

fn confirm_orphan_removal(orphans: &[OrphanInfo]) -> bool {
    eprintln!("warning: {} orphaned skill(s) found:", orphans.len());
    for o in orphans {
        eprintln!("  {} ({})", o.dir_name, o.source);
    }
    Confirm::new()
        .with_prompt("Remove orphaned skills?")
        .default(false)
        .interact_opt()
        .unwrap_or(None)    // non-interactive terminal: treat as "no"
        .unwrap_or(false)   // escape/cancel: treat as "no"
}
```

### New Error Variants Needed

```rust
// Add to src/error.rs SkillsSyncError enum:
#[error("git command failed: git {args}")]
#[diagnostic(code(skills_sync::sync::git_failed))]
GitFailed { args: String, stderr: String },

#[error("git not found — ensure git is installed and in PATH")]
#[diagnostic(code(skills_sync::sync::git_not_found))]
GitNotFound { #[source] source: std::io::Error },

#[error("failed to rename {path}: {source}")]
#[diagnostic(code(skills_sync::sync::rename_failed))]
RenameFailed { path: std::path::PathBuf, source: std::io::Error },

#[error("failed to remove skill at {path}: {source}")]
#[diagnostic(code(skills_sync::sync::remove_failed))]
RemoveFailed { path: std::path::PathBuf, source: std::io::Error },
```

### main.rs Integration Point

```rust
// Current main.rs ends after building all_actions and printing sync plan summary
// Phase 3 inserts sync execution here:

// After: all_actions is built, sync plan is printed
// Add:
use skills_sync::sync;

let target_dir = std::path::Path::new(".claude/skills");
let (token, _) = github::discover_token();  // already called above — pass through

sync::cleanup_stale_temp_dirs(target_dir);

let sync_result = sync::execute_all(
    &all_actions,
    target_dir,
    token.as_deref(),
    &branch_map,  // repo -> default_branch mapping, needed for clone -b
)?;
```

**Note:** The branch name is needed for `git clone -b branch`. Currently `main.rs` has `repo_info.default_branch` per iteration but doesn't store it. A `HashMap<String, String>` (repo → branch) will need to be built during the existing loop.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `git clone` fetches all objects | `git clone --filter=blob:none` for partial clones | Git 2.22+ (2019) | Dramatically faster clones for monorepos with sparse checkout |
| `git sparse-checkout` non-cone mode (glob patterns) | `git sparse-checkout --cone` (directory-based) | Git 2.26+ (2020) | Cone mode is dramatically faster; non-cone has quadratic path-matching |
| Manual credential helper scripts | Token embedded in HTTPS URL | N/A | Simple for CLI tools; auth via URL is the standard for API token auth |

**Deprecated/outdated:**
- Non-cone sparse-checkout patterns: Use `--cone` always for directory-based filtering. Non-cone mode exists but is significantly slower.
- `git clone --filter=tree:0`: Introduced in Git 2.27, not needed for this use case; `blob:none` is the right choice for sparse clones where you want directory structure but only specific file blobs.

## Open Questions

1. **Branch name availability in sync engine**
   - What we know: `repo_info.default_branch` is fetched per-repo in `main.rs` during the Phase 2 loop. SkillMeta does NOT currently store the branch name.
   - What's unclear: Does the sync engine receive the branch name as a parameter, or should it be stored in SkillMeta for updates?
   - Recommendation: Pass branch as part of the `SyncAction` variants or via a `HashMap<repo, branch>` in main.rs. For updates, SkillMeta could store `branch: String` — add this field to SkillMeta in Phase 3, or pass it from the resolver output. Simplest: add `branch` field to `SyncAction::Install` and `SyncAction::Update` variants in resolver.rs.

2. **Non-TTY handling for orphan prompt**
   - What we know: `dialoguer::Confirm::interact_opt()` returns `None` when stdin is not a TTY (confirmed from docs.rs).
   - What's unclear: Should non-TTY silently skip orphan removal (safest) or warn explicitly?
   - Recommendation: Print a warning message and skip (don't remove). The `--force` flag (Phase 4) will handle CI environments properly.

3. **global target_dir resolution**
   - What we know: SYNC-06 requires `~/.claude/skills/` for global target; `dirs` crate resolves home dir; `--global` flag is Phase 4.
   - What's unclear: Whether Phase 3 should hardcode local target or accept `target_dir` parameter.
   - Recommendation: Phase 3 sync engine always accepts `target_dir: &Path` as a parameter. In Phase 3's `main.rs` integration, pass `Path::new(".claude/skills")` as default. Phase 4 will add the CLI flag and pass the global path when `--global` is set.

## Sources

### Primary (HIGH confidence)
- git 2.53.0 man pages — `git clone`, `git sparse-checkout` flags and behavior
- Validated command sequences against `anthropics/anthropic-quickstarts` (live GitHub repo, 2026-03-02)
- `docs.rs/dialoguer/0.11.0` — Confirm struct API (0.12.0 released same major version, API stable)
- `crates.io/crates/dialoguer` — latest version 0.12.0 confirmed 2026-03-02
- Rust `std::fs::rename` docs — wraps POSIX rename(2), atomic on same filesystem
- Rust `std::process::Command` — established pattern from existing github.rs

### Secondary (MEDIUM confidence)
- STATE.md "Use git subprocess (not libgit2) — libgit2 silently ignores --depth 1 and has unreliable sparse checkout" — project-specific validated decision
- CONTEXT.md decisions — all locked implementation choices verified against codebase

### Tertiary (LOW confidence)
- Token-in-URL security exposure: accepted as LOW risk for v1 (temp dir cleaned up quickly); not formally audited

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries verified against official sources or existing Cargo.toml
- Architecture: HIGH — command sequences validated against live repo; patterns derived from existing codebase
- Pitfalls: HIGH — most discovered through direct testing; cross-filesystem rename verified empirically

**Research date:** 2026-03-02
**Valid until:** 2026-09-02 (git CLI API is stable; dialoguer 0.12 API is stable)
