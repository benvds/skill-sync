# Pitfalls Research

**Domain:** Rust CLI tool — GitHub API sync, git operations, local file state management
**Researched:** 2026-02-28
**Confidence:** HIGH (GitHub API limits verified against official docs; git behavior from official git docs; Rust-specific issues from official crate issue trackers)

## Critical Pitfalls

### Pitfall 1: Ignoring the `truncated` Flag from GitHub Trees API

**What goes wrong:**
The project uses `GET /repos/{owner}/{repo}/git/trees/{sha}?recursive=1` to detect repo structure. GitHub silently truncates this response when a repo exceeds 100,000 entries or 7 MB. The `truncated: true` field in the JSON body signals this — but if the code doesn't check it, the tool will operate on an incomplete tree and silently miss skill directories in large repos. The user sees "no skills found" but gets no error.

**Why it happens:**
The happy path works for 99% of repos (small skill repos). Developers test with their own repos, which are small. Truncation only surfaces with unusually large monorepos. It looks like a successful API call.

**How to avoid:**
Always check `response.truncated` before parsing the tree. If `true`, fall back to non-recursive traversal: fetch the root tree, identify `blob`/`tree` entries at depth 0, then fetch each relevant subtree individually. Document this behavior in code with a comment pointing to the GitHub API limit.

**Warning signs:**
- A known skill repo reports zero skills on sync
- Tree response has exactly 100,000 entries (at the limit)
- Integration test with a large repo finds fewer skills than expected

**Phase to address:**
GitHub API integration phase (core sync implementation). Build the truncation check before shipping any tree-traversal logic.

---

### Pitfall 2: Unauthenticated GitHub API Calls Hit Rate Limit at 60 Requests/Hour

**What goes wrong:**
Without a `Authorization: Bearer <token>` header, GitHub REST API allows only 60 requests per hour per IP. A user with 10+ repos in their `.skills.toml` can hit this limit in a single sync run (structure fetch + HEAD SHA fetch per repo). The tool fails mid-sync with a cryptic 403 or 429 response. On shared networks (offices, CI), multiple users exhaust a shared IP pool even faster.

**Why it happens:**
Developers build the happy path first without auth. It works in dev with a few repos. The limit only bites users with larger manifests or on shared infrastructure.

**How to avoid:**
Authenticate every request. On startup, check for a `GITHUB_TOKEN` environment variable, fall back to `gh auth token` (GitHub CLI credential helper) via subprocess call. If neither is present, warn prominently before making any API calls. Never make an unauthenticated request to the GitHub API — even with one repo in the manifest, unauthenticated HEAD SHA checks for update detection accumulate fast.

**Warning signs:**
- Rate limit errors appear intermittently (depends on timing of reset)
- Works for users who happen to have `GITHUB_TOKEN` set
- CI fails but local works

**Phase to address:**
GitHub API integration phase. Authentication must be implemented before any other API call logic.

---

### Pitfall 3: Missing User-Agent Header Causes Silent 403 Errors

**What goes wrong:**
GitHub API requires a `User-Agent` header. Requests without it return HTTP 403 with the message "Please make sure your request has a User-Agent header." The `reqwest` crate does NOT automatically set a User-Agent for API clients (only the blocking client sometimes does). This produces a 403 that looks like an auth failure, and error messages from JSON deserialization of the error body can be misleading.

**Why it happens:**
Developers used to browser clients or curl assume HTTP clients set a User-Agent automatically. The `reqwest` default does not include one for programmatic use. The 403 response body contains the real explanation, but if the error handler doesn't parse the GitHub error message body, the error surfaces as an opaque HTTP 403.

**How to avoid:**
Set `User-Agent: skills-sync/{version}` as a default header on the `reqwest::Client` builder when constructing the HTTP client — not per-request. Include the tool name and version so GitHub can contact the developer if the integration causes issues (this is GitHub's stated requirement). Add an integration test that verifies a real API call succeeds.

**Warning signs:**
- All API calls fail with 403 regardless of authentication
- Error message does not mention rate limiting
- curl with the same token succeeds but the tool fails

**Phase to address:**
GitHub API integration phase, during HTTP client setup.

---

### Pitfall 4: Shallow Clone + Sparse Checkout Combination Breaks Update Detection

**What goes wrong:**
The project uses shallow clones (`--depth 1`) for single-skill repos and sparse checkout for monorepo skill extraction. These two features interact poorly. When doing `git fetch --depth 1` on a shallow clone to check for updates, git may download significantly more history than expected. Additionally, sparse checkout state is not preserved across fetch operations without explicit reconfiguration. After an update fetch on a sparse-checkout clone, the working tree may include files outside the intended skill directory, or the sparse-checkout patterns may be reset.

**Why it happens:**
Shallow clones and sparse checkout are each well-documented individually but rarely combined in tooling. The mental model (depth + pattern = minimal checkout) doesn't map cleanly to git's internal reflog and object model. Update operations on shallow clones are particularly tricky because `git fetch --depth 1` on an already-shallow clone can trigger history re-evaluation.

**How to avoid:**
For monorepo skill extraction (sparse checkout), prefer the `git clone --no-checkout --filter=blob:none --depth 1` pattern followed by `git sparse-checkout set --cone <skill-dir>` and then `git checkout`. For updates, explicitly re-apply sparse-checkout patterns after fetch. Use cone mode (directory-level) rather than non-cone mode (gitignore patterns) for O(1) matching vs O(N*M). Consider storing the expected skill directory path in the `.skills-sync-meta.json` file to reconfigure sparse checkout on update.

**Warning signs:**
- Update operations are slower than fresh installs
- After update, skill directory contains unexpected files
- Sparse checkout state appears reset after update

**Phase to address:**
Git operations phase. Design the update flow (not just install) before coding either operation.

---

### Pitfall 5: Metadata File Written Before Git Operation Completes

**What goes wrong:**
If `.skills-sync-meta.json` is written before the git clone or sparse checkout finishes, a crash, network failure, or SIGINT between the two steps leaves state inconsistent: metadata says "installed at SHA X" but skill files are incomplete or absent. On the next run, update detection compares local SHA (from metadata) against remote HEAD and sees "up to date" — orphan detection won't flag it either, because the metadata file exists. The skill silently disappears from the user's perspective.

**Why it happens:**
It feels natural to write metadata at the start of an operation to "claim" the slot and prevent re-entry. The correct pattern (write-on-commit, not write-on-start) requires more careful sequencing.

**How to avoid:**
Write `.skills-sync-meta.json` only after a successful git operation, as the last step. Use an atomic write pattern: write to a temp file in the same directory, then `rename()` (which is atomic on POSIX). On failure, clean up the partial skill directory rather than leaving it. On startup, validate that metadata-declared skills have their expected directory structure present; if not, treat them as re-installable.

**Warning signs:**
- Skills appear installed but directories are missing or empty
- `--dry-run` shows no changes but skills don't work
- Users report "skill installed" but can't use it

**Phase to address:**
State management phase. Design the write ordering and recovery logic before implementing any sync operations.

---

### Pitfall 6: No SIGINT Handling Leaves Partial State on Ctrl+C

**What goes wrong:**
If the user presses Ctrl+C during a clone operation, the git subprocess is killed but the partial clone directory remains. On the next run, the partial directory exists but is not a valid git repo. `git` operations against it fail. If the metadata file wasn't written (see Pitfall 5), orphan detection won't notice the partial directory either, because it only looks for metadata. The directory accumulates as silent garbage.

**Why it happens:**
Rust's `std::process::Command` does not automatically kill child processes when the parent receives SIGINT. The child (git) inherits the signal on Unix, but the parent exits before cleanup. On Windows, behavior differs further.

**How to avoid:**
Register a SIGINT handler using the `signal-hook` or `ctrlc` crate. In the handler, set an `Arc<AtomicBool>` flag. Between each major operation step, check the flag. If set, roll back: remove partial clone directories and exit cleanly. Alternatively, write a `.skills-sync-partial` sentinel file at the start of each install, remove it on success — a startup scan for sentinel files triggers cleanup.

**Warning signs:**
- Directories exist in the skills folder with no corresponding metadata
- Users report errors after interrupting a sync
- `git status` inside installed skill dirs reports not a git repo

**Phase to address:**
State management phase, alongside atomic write design.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Skip authentication check, rely on env var being set | Simpler code, faster initial build | Users hit 60 req/hr limit; broken on fresh machines | Never — auth detection is a one-time build cost |
| Use `fs::write()` for metadata instead of atomic write | Fewer dependencies | Silent corruption on crash/interrupt mid-write | Never — corrupted state is a user-visible bug |
| Hardcode `~/.claude/skills` as the only target | Simpler path logic | Breaks multi-tool users; ignores local vs global distinction | Never — manifest already specifies this is configurable |
| Run git as subprocess with `git` in PATH | No libgit2 dep, system version used | Path differences on Windows; git may not be in PATH | Acceptable if startup checks for `git` and fails fast with clear error |
| Call GitHub API without checking `x-ratelimit-remaining` | No header parsing code | Next call fails unexpectedly; no early warning to user | Never — header parsing is one field check |
| Write full tree fetch results to memory without truncation check | Simpler parsing | Silent data loss for large repos | Never — truncation check is a single boolean field |

---

## Integration Gotchas

Common mistakes when connecting to external services.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| GitHub REST API | No User-Agent header | Set `User-Agent: skills-sync/{version}` on `reqwest::Client` builder |
| GitHub REST API | Not checking `x-ratelimit-remaining` before the next call | Read rate limit headers after every response; warn when remaining < 10 |
| GitHub REST API | Treating 403 as always an auth error | Parse error body: 403 can mean rate-limited, bad User-Agent, or bad token |
| GitHub REST API | Ignoring `retry-after` header on 429 | Respect `retry-after` seconds exactly; use exponential backoff if absent |
| GitHub Trees API | Not checking `truncated: true` in response | Always gate tree parsing on a truncation check; fall back to non-recursive traversal |
| git subprocess | Assuming git is on PATH on all platforms | Check for git binary at startup and emit a clear "git not found, please install git" error |
| git subprocess | Not handling non-zero exit codes | Capture stderr from git subprocess; include it verbatim in error messages |
| git subprocess | Using full clone for update detection | Use the GitHub API to compare remote HEAD SHA to local metadata; only clone when a change is confirmed |
| `gh` CLI credential helper | `gh` not installed on fresh machines | Treat `gh` auth as optional fallback; `GITHUB_TOKEN` env var is primary; document both |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Fetching full repo HEAD via clone to check for updates | Every `sync` does a full shallow clone even when skills are current | Compare local SHA in `.skills-sync-meta.json` against remote HEAD via API before cloning | At 5+ repos: each full check adds 2-10s |
| Making API calls sequentially without reading rate-limit headers | After 5000 calls/hour (authenticated), tool silently fails mid-sync | Read `x-ratelimit-remaining` and warn/pause proactively | At 1000+ repos (power users with many manifest entries) |
| Not caching the resolved default branch name | Extra API call per repo to resolve default branch before tree fetch | Store resolved branch name in `.skills-sync-meta.json` at install time | At 10+ repos: unnecessary API call per repo per sync |
| Spawning git subprocess per skill in sequence | Long total sync time for manifests with many repos | Parallelize git operations (tokio tasks or rayon); rate-limit to avoid secondary limits | At 10+ skills: noticeable latency difference |

---

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Logging or printing the GitHub token in error messages | Token exposure in terminal history, CI logs, or bug reports | Never include token value in any user-facing output; log "token present: yes/no" only |
| Storing the GitHub token in `.skills.toml` or `.skills-sync-meta.json` | Token committed to dotfiles repos, leaked in file sync | Read token only from environment variables or system credential store; document this explicitly |
| Blindly executing content from cloned skill repos | Malicious SKILL.md could contain shell injection if tool ever evaluates content | For v1: never execute or evaluate skill file contents, only copy them. Add note to defer skill validation |
| Writing cloned content outside the designated skills directory | Path traversal via malicious repo with `../` in skill dir name | Canonicalize all destination paths and verify they are children of the target skills dir before write |
| No validation that `clone_url` in API response is a GitHub URL | If response is tampered (MITM), could redirect clone to arbitrary server | Verify clone URLs match `github.com` domain before passing to git subprocess |

---

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Printing raw git stderr output verbatim | Users see "fatal: repository '...' not found" without context about which skill or why | Wrap git errors: "Failed to clone skill 'my-skill' from repo 'user/repo': repository not found. Check the repo name in your .skills.toml." |
| No progress feedback during clone operations | Users see nothing for 5-30 seconds; assume tool crashed | Use `indicatif` progress bar with repo name; show "Cloning user/repo..." during each operation |
| `--dry-run` that still makes API calls | Users on metered connections or with low rate limits are surprised | Dry-run should only skip file system changes; API reads (to determine what would change) are acceptable, but document this |
| Destroying skills without user confirmation | Users accidentally `--force` and lose local customizations | Default to prompting for destructive operations (removal, overwrite of modified files). `--force` skips prompt, not safety checks |
| Orphan detection that lists orphans but doesn't explain how to dismiss them | Users don't know how to permanently ignore a skill | When listing orphans, show the command to either remove or add to a `[ignored]` section in `.skills.toml` |
| Config file not found error with no suggestion | Users new to the tool get a cryptic "No .skills.toml found" | Suggest `skills-sync --init` to generate a starter config, and show the expected search path |
| Silent success when nothing changed | Users run sync and see blank output, unsure if it worked | Always print a summary: "3 skills up to date, 0 installed, 0 updated, 0 orphaned" even when nothing changed |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **GitHub API integration:** Looks done after first successful request — verify User-Agent header is set, auth token is threaded through all requests, rate limit headers are read and respected
- [ ] **Shallow clone install:** Looks done after first clone succeeds — verify the `.skills-sync-meta.json` is written after (not before) the clone completes, and that Ctrl+C leaves a clean state
- [ ] **Sparse checkout for monorepos:** Looks done when subdirectory appears — verify cone mode is used (not non-cone), that the checkout contains only the skill subdirectory, and that update operations re-apply the pattern
- [ ] **Orphan detection:** Looks done when orphans are listed — verify detection is based on metadata file presence, not directory naming convention, and that the prompt works correctly with `--force` and `--dry-run` flags
- [ ] **Update detection:** Looks done when SHA comparison works — verify that a missing metadata file triggers a fresh install (not an error), and that a missing remote repo is handled gracefully
- [ ] **Tree API truncation:** Looks done when repos list correctly — verify the `truncated` field is checked and there is a fallback path, not just a silent partial result
- [ ] **`--dry-run` flag:** Looks done when output is printed — verify no file system writes occur, no directories are created, and no git processes are spawned
- [ ] **Cross-platform paths:** Looks done on macOS — verify path separator handling on Windows (use `std::path::PathBuf` throughout, never string concatenation with `/`)

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Partial clone from SIGINT | LOW | Delete partial directory; re-run sync |
| Corrupted metadata file | LOW | Delete `.skills-sync-meta.json` for the affected skill; re-run sync treats it as not installed |
| Rate limit hit mid-sync | LOW | Wait for reset window (check `x-ratelimit-reset` header timestamp); re-run sync |
| Truncated tree response missed, wrong skills installed | MEDIUM | User must manually inspect skills dir; re-run sync after fix is shipped |
| Token logged in error output | HIGH | User must rotate GitHub token; audit log for exposure |
| Path traversal via malicious repo | HIGH | Remove all installed skills from affected repo; validate all existing skill paths |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Trees API truncation | GitHub API integration | Integration test with mocked truncated response; assert fallback is triggered |
| Unauthenticated rate limits | GitHub API integration | Test with no token present; verify warning is printed and tool either exits or prompts |
| Missing User-Agent | GitHub API integration | Inspect outgoing request headers in tests; assert `User-Agent` is present |
| Shallow + sparse checkout interaction | Git operations implementation | Test update cycle on a freshly installed monorepo skill; verify sparse pattern survives fetch |
| Metadata written before clone completes | State management design | Unit test: inject failure mid-clone; verify metadata file does not exist after failure |
| SIGINT leaves partial state | State management design | Integration test: kill tool mid-install; verify cleanup occurs on next run |
| Verbatim git errors in UX | CLI UX layer | Manual test with invalid repo URL; verify error message names the skill and explains the failure |
| No progress feedback | CLI UX layer | Manual test with a slow network; verify progress indicators appear |
| Token in error output | GitHub API integration | Code review gate: grep for token variable names in error/log formatting code |

---

## Sources

- [GitHub REST API: Rate limits for the REST API](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api) — PRIMARY, verified
- [GitHub REST API: Best practices](https://docs.github.com/rest/guides/best-practices-for-using-the-rest-api) — PRIMARY, verified
- [GitHub REST API: Git Trees endpoint](https://docs.github.com/en/rest/git/trees) — PRIMARY, verified (100k entries / 7MB limit, truncated field)
- [GitHub Community: Best Practices for Handling GitHub API Rate Limits](https://github.com/orgs/community/discussions/151675) — MEDIUM
- [rust-cookbook issue: User-Agent header missing causes 403](https://github.com/rust-lang-nursery/rust-cookbook/issues/662) — MEDIUM
- [git-sparse-checkout official docs](https://git-scm.com/docs/git-sparse-checkout) — PRIMARY, verified (cone vs non-cone, O(N*M) non-cone cost)
- [GitHub Blog: Bring your monorepo down to size with sparse-checkout](https://github.blog/open-source/git/bring-your-monorepo-down-to-size-with-sparse-checkout/) — MEDIUM
- [Rust CLI book: Signal handling](https://rust-cli.github.io/book/in-depth/signals.html) — HIGH
- [atomicwrites crate](https://crates.io/crates/atomicwrites) — HIGH (write-to-temp-then-rename pattern)
- [reqwest: git2-rs clone 5x slower than native git](https://github.com/rust-lang/git2-rs/issues/729) — MEDIUM (validates subprocess-git approach)
- [skills-cli by dhruvwill: naming conflicts, path detection issues](https://github.com/dhruvwill/skills-cli) — LOW (real-world comparable tool)
- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/) — MEDIUM (dry-run, UX patterns)
- [Rust CLI book: Error reporting](https://rust-cli.github.io/book/tutorial/errors.html) — HIGH

---
*Pitfalls research for: Rust CLI GitHub skill sync tool (skills-sync)*
*Researched: 2026-02-28*
