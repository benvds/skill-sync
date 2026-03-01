# Pitfalls Research

**Domain:** Thin CLI wrapper / orchestration tool (skills-sync delegating to `npx skills`)
**Researched:** 2026-03-01
**Confidence:** MEDIUM — Core pitfalls verified via official docs, GitHub issues, and CLI tooling best practices. `npx skills` lock file internals verified via DeepWiki/GitHub source analysis.

---

## Critical Pitfalls

### Pitfall 1: Reading an Undocumented, Mutable Third-Party Lock File as Ground Truth

**What goes wrong:**
skills-sync reads `~/.agents/.skill-lock.json` directly to determine what is installed, treating it as a stable source of truth. The format is internal to the `skills` package, not officially documented as a public API, and has already changed between versions (format was "wiped" when a version bump occurred — users had to reinstall all skills). If Vercel changes the structure without a semver major bump, skills-sync silently misreads installed state, produces wrong diffs, and could delete or re-install skills incorrectly.

**Why it happens:**
The lock file is the fastest way to read installed state. `npx skills list` is slow (spawns a subprocess, waits for output parsing). The lock file is a tempting shortcut.

**How to avoid:**
Treat `~/.agents/.skill-lock.json` as a hint, not ground truth. Add a version field check at startup: read the `version` (or equivalent schema marker) from the lock file and fail loudly if it does not match the expected value. Pin the tested lock file version in code comments. Provide a fallback to `npx skills list` output when the lock file version is unrecognized. Never write to this file — read only.

**Warning signs:**
- No schema version check in the file-reading code
- Diffs producing "orphan" skills that are clearly still installed
- Users reporting skills being removed and re-added on every sync run
- Vercel releases a new `skills` version with a changelog entry about lock file changes

**Phase to address:**
Lock file reading phase (the diff computation step). Validate schema before parsing. Write a dedicated `readLockFile()` function with an explicit `EXPECTED_LOCK_VERSION` constant that throws a descriptive error when mismatched.

---

### Pitfall 2: Project-Scoped Installs Are Invisible to the Global Lock File

**What goes wrong:**
`~/.agents/.skill-lock.json` only tracks globally installed skills. Project-scoped installs (`npx skills add` without `-g`) are not written to the lock file at all. This is a confirmed, active limitation in the `skills` tool (GitHub issue #337, closed Feb 2026 in favor of issue #155, marked "WIP"). If skills-sync uses only the global lock to detect orphans, it will misidentify all project-scoped skills as "not installed" and either prompt to remove them or silently ignore the mismatch.

**Why it happens:**
The `addSkillToLock()` function in `skills`'s source only runs when `installGlobally === true`. Project installs write files to `./[agent]/skills/` but never update the lock file. This is a gap in the upstream tool, not a bug in reading.

**How to avoid:**
skills-sync's design already addresses this with `.skills-sync-sources.json` as a project-scoped tracking file — that decision is correct and essential. Do not rely on the global lock file for project-scope state. The project tracking file must be treated as the authoritative source for `--local` scope. Carefully document what each scope uses as its source of truth.

**Warning signs:**
- `--local` sync always reports all skills as missing on first run
- Orphan detection incorrectly removes project-scoped skills
- Tests that only exercise global scope pass while local scope is broken

**Phase to address:**
Architecture phase — the `.skills-sync-sources.json` design is the mitigation. Diff computation must branch on scope: global lock file for `--global`, project tracking file for `--local`.

---

### Pitfall 3: Swallowing `npx skills` Errors — Output Mixing and Exit Code Misinterpretation

**What goes wrong:**
When delegating to `npx skills add` or `npx skills remove`, the subprocess may fail (non-zero exit, errors on stderr) while skills-sync reports success. This happens when: (a) stderr is captured but not re-emitted, (b) exit code is checked but only as a boolean, (c) `npx` itself exits 0 even though the inner command failed (a known npx behavior in some versions). Users see "sync complete" when skills were not actually installed.

**Why it happens:**
Wrapper tools often capture stdout to parse results, accidentally swallowing stderr. `npx` has historically had exit code propagation issues. Developers focus on the happy path and assume non-zero exit means a recoverable user error rather than a process-level failure.

**How to avoid:**
Use `execa` (not raw `child_process`) with explicit `{ stdout: 'inherit', stderr: 'inherit' }` or `{ all: true }` to surface all subprocess output to the terminal. Always check both exit code and the presence of error output. Do not merge stdout and stderr — keep them separate so skill installation output goes to stdout and errors are clearly on stderr. Re-emit subprocess stderr even when capturing for parsing. In `--verbose` mode, show the raw subprocess output.

**Warning signs:**
- skills-sync shows "Installed X skills" but the skill files do not appear on disk
- No visible error when running with a bad source URL
- `--dry-run` output is accurate but actual run silently fails

**Phase to address:**
Shell-out execution layer. Write a `runSkillsCommand()` wrapper with explicit error handling as the first implementation step — before connecting to diff logic.

---

### Pitfall 4: npx Stale Cache Causes Non-Deterministic Behavior

**What goes wrong:**
`npx skills` caches packages. When Vercel releases a new version of `skills`, users may continue running an older cached version without knowing it. This means: (a) the lock file format skills-sync reads may be written by a different version than the one running, (b) behavior differences between versions are invisible, (c) `--check` in CI may pass locally but fail in clean CI environments that always pull fresh.

**Why it happens:**
`npx` with no version pin resolves to "latest" but caches aggressively. This is a confirmed bug in npm/npx (multiple open issues as of 2025/2026). Different machines may have different cached versions even with the same `npx skills` invocation.

**How to avoid:**
Document in skills-sync's README that users should clear the npx cache if behavior seems wrong. When delegating, consider using `npx skills@latest` explicitly in the spawned command to force fresh resolution. In CI (when `--yes` flag is present), always force-fresh with `--prefer-online` or equivalent. This is a partial mitigation — npx caching is ultimately the upstream's problem.

**Warning signs:**
- "Works on my machine" sync failures in CI
- Lock file format mismatches after Vercel releases new `skills` version
- `npx skills list` returning stale data

**Phase to address:**
CI documentation and the `--yes` flag implementation. Document the `npx` cache issue prominently in the README. Do not try to solve it programmatically — just mitigate and document.

---

### Pitfall 5: TTY / Interactive Prompt Passthrough Breaks When Piped

**What goes wrong:**
`npx skills add` shows interactive prompts (skill selection, agent selection, confirmation). When skills-sync spawns it as a subprocess, these prompts fail because the subprocess is not connected to a real TTY. Without `--yes` / `-y` flags, the delegated process may hang waiting for stdin that never comes, or crash with a "not a tty" error. This is especially bad in CI where the process hangs indefinitely.

**Why it happens:**
Node.js detects TTY context via `process.stdout.isTTY`. When spawned as a subprocess with piped stdio, `isTTY` is false and interactive libraries like `inquirer` behave differently — some hang, some throw, some silently skip prompts.

**How to avoid:**
Always pass `-y` (skip confirmations) and `-a [agent]` (no agent selection prompt) when invoking `npx skills add` from skills-sync. The `--agent` flag passthrough in skills-sync directly addresses this. Never rely on interactive mode when delegating. When the user does not provide `--agent`, skills-sync should either prompt the user itself (before spawning) or fail early with a clear message rather than spawning a subprocess that will hang.

**Warning signs:**
- `npx skills-sync` hangs indefinitely during `add` step
- CI jobs timeout on the sync step with no output
- Tests that use subprocess spawning work locally but fail in CI containers

**Phase to address:**
CLI flag design phase and subprocess execution. The `--yes` flag passthrough to `npx skills` and the `--agent` flag resolution must be part of the initial subprocess invocation design, not added later.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Parse lock file without version check | Faster to implement | Silent breakage when Vercel changes format | Never — add version check from day one |
| Use `child_process.exec` instead of `execa` | Fewer dependencies | String-based error handling, no streaming, shell injection risk | Never for this tool |
| Dry-run as "just don't call subprocess" | Simple implementation | If pre-subprocess logic has side effects (file writes), they still run | Only acceptable if the code is clearly separated into "read" and "write" phases |
| Use `npx skills list` output parsing instead of lock file | No undocumented format risk | Slow, fragile (output format not guaranteed stable either) | Acceptable as fallback when lock file version mismatch occurs |
| Trust only project tracking file for local scope | Simpler diff logic | May miss skills installed manually outside skills-sync | Acceptable — document the limitation explicitly |

---

## Integration Gotchas

Common mistakes when connecting to `npx skills` and the lock file.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `npx skills add` | Calling without `-y` and `-a [agent]`, causing hang | Always pass `-y` and resolved `--agent` value |
| `npx skills remove` | Not verifying exit code — remove is silent on success | Check exit code AND verify the skill file is gone from disk after removal |
| `~/.agents/.skill-lock.json` | Reading without checking if file exists | Guard with `fs.existsSync`, return empty state (not error) when missing — new installs have no lock file |
| `~/.agents/.skill-lock.json` | Writing to it (even to "help") | Never write — skills-sync is read-only on this file |
| `.skills-sync-sources.json` | Storing absolute paths | Store relative paths or source identifiers — absolute paths break when the project moves |
| TOML parsing | Using an unmaintained parser (`toml` npm package, last updated for TOML v0.4) | Use `smol-toml` (TOML 1.1.0 compliant) or `@iarna/toml` (TOML 1.0.0) |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Calling `npx skills add` sequentially for each source | Acceptable with 2-3 sources, slow with 10+ | Explore parallel invocations if `npx skills` allows it; document sequential limitation | With 10+ skills, sync takes >30s |
| Reading lock file on every diff check | Fine for a single run | Not applicable here — single invocation tool, not a daemon | N/A for this tool |
| Spawning `npx skills` with no timeout | Hangs forever on network issues | Set a timeout in subprocess options (e.g., 60s per command) | On flaky networks or GitHub rate limits |

---

## Security Mistakes

Domain-specific security issues.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Passing user-provided TOML strings as shell arguments | Shell injection if source URLs contain special characters | Always pass args as array to execa, never string concatenation into shell command |
| Reading arbitrary file paths from `--config` flag without validation | Path traversal — user can point to any file on the system | Validate the path is a file, exists, and has `.toml` extension before reading |
| Logging full skill source URLs in verbose mode | Low risk but may expose internal repo URLs | Acceptable to log, but note for enterprise users with private repo names |

---

## UX Pitfalls

Common user experience mistakes for this type of CLI tool.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Showing raw subprocess output from `npx skills` mixed with own output | User cannot tell what went wrong or what the tool did | Prefix or section subprocess output; show summary separately |
| `--dry-run` output is not actionable (just "would install X") | User cannot validate if the dry-run is correct | Show exact `npx skills` commands that would be run, not just the result |
| Failing silently when `.skills.toml` has a TOML parse error | User thinks sync succeeded but nothing happened | Catch parse errors immediately, print line/column info from TOML parser's error, exit 1 |
| Removing orphaned skills without confirmation in non-`--yes` mode | Destructive action surprises users | Always prompt before removal unless `--yes` is set; list what will be removed first |
| `--check` flag exiting 0 when skills are out of sync | CI gate passes when it should fail | `--check` MUST exit 1 for any out-of-sync condition, including orphans |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **TOML parsing:** Parser selected and installed — verify it handles the `repos = ["owner/repo"]` array syntax and gives line/column on error
- [ ] **Lock file reading:** Reads the file — verify it guards against missing file, corrupt JSON, and unexpected schema version
- [ ] **Diff computation:** Shows correct diff — verify orphan detection works for `--local` scope (project tracking file, not global lock)
- [ ] **`--dry-run` flag:** No subprocess called — verify no file system writes either (`.skills-sync-sources.json` should not be updated in dry-run)
- [ ] **`--check` flag:** Runs sync — verify it exits 1 (not just prints warning) when out of sync
- [ ] **`npx skills add` delegation:** Runs the command — verify `-y` and `-a [agent]` are always passed, never relying on interactive mode
- [ ] **Error handling:** Catches subprocess failure — verify stderr from `npx skills` is visible to the user, not swallowed

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Lock file format changed by Vercel update | MEDIUM | Detect via version check at startup; print clear error message with `npx skills list` as diagnostic; user runs `npx skills list` to verify state manually |
| Subprocess hang (TTY issue, no `--yes`) | LOW | Add timeout to subprocess call; kill after N seconds; print error telling user to add `--agent` flag |
| Wrong skills removed due to scope confusion | HIGH | No automatic recovery; user must re-run `npx skills add` for each removed skill; document the scope behavior clearly |
| TOML parse error in user manifest | LOW | Parser error message with line/column is sufficient; user fixes the file |
| `.skills-sync-sources.json` out of sync with reality | MEDIUM | Provide a `--repair` or `--reset-tracking` flag that regenerates the tracking file from `npx skills list` output |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Lock file schema version not checked | Lock file reading implementation | Unit test: mock a lock file with wrong/unknown version, verify descriptive error |
| Project-scoped installs invisible in global lock | Architecture / diff computation design | Integration test: install a skill locally, verify it appears in diff correctly using project tracking file |
| Subprocess errors swallowed | Subprocess execution layer (first implementation) | Integration test: pass invalid source URL, verify error is visible and exit code is 1 |
| npx stale cache | Documentation + CI flag (`--yes`) | Manual verification: run in Docker with clean npm cache, confirm consistent behavior |
| TTY hang on interactive prompt | Subprocess invocation design | Integration test: run sync in non-TTY environment (pipe to `/dev/null`), verify no hang |
| Dry-run writes side effects | `--dry-run` flag implementation | Unit test: run with `--dry-run`, verify `.skills-sync-sources.json` is not modified |
| `--check` exits 0 when out of sync | `--check` flag implementation | Integration test: create out-of-sync state, verify exit code is 1 |

---

## Sources

- [vercel-labs/skills GitHub repo](https://github.com/vercel-labs/skills) — lock file location, global vs project scope behavior (MEDIUM confidence)
- [GitHub Issue #337: project-scoped npx skills update not working](https://github.com/vercel-labs/skills/issues/337) — confirmed project scope not tracked in lock file (HIGH confidence, direct upstream issue)
- [DeepWiki: vercel-labs/skills Getting Started](https://deepwiki.com/vercel-labs/skills/2-getting-started) — lock file structure and update detection mechanism (MEDIUM confidence)
- [smol-toml — TOML 1.1.0 compliant parser](https://github.com/squirrelchat/smol-toml) — TOML version compatibility (HIGH confidence, official repo)
- [TOML v1.1.0 spec — inline tables, trailing commas](https://toml.io/en/v1.1.0) — TOML edge cases (HIGH confidence, official spec)
- [execa GitHub repo](https://github.com/sindresorhus/execa) — subprocess streaming, TTY, Windows compat (HIGH confidence, official repo)
- [Node.js TTY docs](https://nodejs.org/api/tty.html) — TTY detection in child processes (HIGH confidence, official docs)
- [Node.js child_process docs](https://nodejs.org/api/child_process.html) — cross-platform subprocess pitfalls (HIGH confidence, official docs)
- [npx stale cache bug reports](https://github.com/npm/cli/issues/4108) — stale version caching in npx (HIGH confidence, official npm/cli issues)
- [CLI Guidelines — stderr/stdout separation](https://clig.dev/) — subprocess output best practices (HIGH confidence, widely cited reference)
- [process.exit() vs throw — async cleanup](https://humanwhocodes.com/blog/2014/02/04/maintainable-node-js-javascript-avoid-process-exit/) — Node.js process exit pitfalls (MEDIUM confidence, established blog)
- [MCP Python SDK — shell=True injection risk](https://github.com/modelcontextprotocol/python-sdk/issues/1257) — shell injection via subprocess (HIGH confidence, official issue)

---
*Pitfalls research for: thin CLI wrapper / orchestration tool (skills-sync)*
*Researched: 2026-03-01*
