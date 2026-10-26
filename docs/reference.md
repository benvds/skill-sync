# Configuration and CLI reference

Declarative [Agent Skills](https://agentskills.io/specification) installation for multiple AI agents. Maintain one personal manifest, add project-specific manifests, and synchronize independent copies with state tracked per skill and destination.

## Quick start

```sh
cargo install --path .

# Create the central personal configuration
skill-sync --global --init
# Edit ~/.config/skill-sync/skills.toml, then:
skill-sync --global

# Add project-specific skills
cd /path/to/project
skill-sync --init
# Edit .skills.toml, then:
skill-sync
```

A normal run processes the central user configuration and the nearest project manifest, if present. `--global` processes only the central configuration; `--local` processes only the project manifest. An explicit `--config` processes exactly that file.

## Central configuration

The central configuration is `$XDG_CONFIG_HOME/skill-sync/skills.toml`, or `~/.config/skill-sync/skills.toml` when `XDG_CONFIG_HOME` is unset. This convention also applies on macOS. Its lock file is `skills.lock` in the same directory.

```toml
[sync]
scope = "user"
agents = ["codex", "claude-code", "cursor", "gemini-cli", "copilot", "opencode"]
mode = "copy"

[skills]
repos = [
    "anthropics/skills/skills/skill-creator",
    "github/awesome-copilot/skills/github-issues",
]
```

Each listed skill is copied to every selected agent's user skills directory. `copy` is the default and currently the supported installation mode: changing one copy does not alter another copy or the source. No symlinks to a temporary checkout or to your personal home directory are installed.

For backward compatibility, central discovery also checks the previous platform-specific config directory (for example `~/Library/Application Support/skill-sync/skills.toml` on macOS), then `~/.agents/skills/.skills.toml` and `~/.agents/skills/skills.toml`. The first existing central configuration wins; these files are not merged. `--global --init` creates the new central path.

## Project configuration

Add `.skills.toml` at your project root and commit it with your project:

```toml
[sync]
scope = "project"
agents = ["codex", "claude-code", "cursor"]
mode = "copy"

[skills]
repos = [
    "your-org/project-skills",
    "your-org/team-skills/skills/deploy",
]
```

These skills are installed only inside that project. Personal skills remain in your home directory; they are not copied into each repository. Project manifests have their own source lists, agent selections and locks. Settings and source lists are not implicitly inherited or merged with the personal manifest.

Discovery walks upward from the working directory and uses the nearest `.skills.toml` or `skills.toml` (the hidden name wins at the same level). It stops at a Git repository/worktree root or your home directory. This allows a monorepo package to have its own manifest. One run processes the nearest project manifest, not every ancestor manifest.

The lock stays beside its manifest: `.skills.toml` → `.skills.lock`, `skills.toml` → `skills.lock`. A copied project lock is rebased to the current checkout/worktree before use, so it cannot prune installations in the original checkout. The lock contains local absolute target paths; it is rewritten when a copied checkout is synchronized.

## Agent directories

These are the native destinations selected by `agents`. Paths in the project column are relative to the manifest directory.

| Agent ID | User destination | Project destination |
|---|---|---|
| `codex` | `~/.agents/skills` | `.agents/skills` |
| `claude-code` | `~/.claude/skills` | `.claude/skills` |
| `cursor` | `~/.cursor/skills` | `.cursor/skills` |
| `gemini-cli` | `~/.gemini/skills` | `.gemini/skills` |
| `copilot` / `github-copilot` | `~/.copilot/skills` | `.github/skills` |
| `opencode` | `$XDG_CONFIG_HOME/opencode/skills` (default `~/.config/opencode/skills`) | `.opencode/skills` |
| `goose` / `universal` | `~/.agents/skills` | `.agents/skills` |
| `kiro` | `~/.kiro/skills` | `.kiro/skills` |

Agent IDs are explicit: skill-sync installs to selected agents even if their application is not installed. Unknown IDs fail before installation. Agents resolving to the same physical destination, including symlink aliases, trigger one copy and one lock entry per skill.

Many agents also discover skills in other agents' directories. Identical names across user/project scopes or native/shared locations may therefore be visible more than once; the agents determine their own precedence. skill-sync warns when its selected manifests declare the same skill name. It does not promise that a project skill overrides a personal skill. Choose unique names or limit the agent selection when necessary. Local copying does not upload skills to ChatGPT web or other cloud runtimes.

### Custom destinations

Keep using `~/.codex/skills`, or choose another path for a supported agent:

```toml
[sync]
scope = "user"
agents = ["codex", "claude-code"]

[agents.codex]
user_dir = "~/.codex/skills"
project_dir = ".team/skills"

[skills]
repos = ["anthropics/skills/skills/skill-creator"]
```

`~/` expands to the current home directory. Other relative paths are relative to the manifest directory. Project destinations must remain inside that directory after symlinks and `..` have been resolved. An explicit project-scoped manifest also keeps `--target` inside its directory. A legacy manifest without `[sync]` can use an external `--target` as a user-scoped compatibility configuration. `--agent` overrides the selected agents for a run. `--target` overrides all configured destinations with one explicit directory, relative to the working directory; select a single manifest with `--local` or `--config` when both scopes exist.

```sh
skill-sync --local --agent codex --agent claude-code
skill-sync --global --agent codex,cursor
skill-sync --config /path/to/skills.toml --target ~/.codex/skills
```

### Existing skills collections

Old manifests containing only `[skills]` remain supported. Without an agent selection, a user configuration targets `~/.agents/skills`; a project/explicit manifest retains directory detection:

| Manifest directory | Destination |
|---|---|
| Any directory named `skills` | That directory directly, even if it is a Git repository |
| `.agents`, `.codex`, `.claude`, `.cursor`, `.gemini`, `.copilot`, `.github`, `.opencode`, `.kiro` | Its `skills` subdirectory |
| Project root (Git/worktree, agent instructions, common build manifest) | `.agents/skills` |
| Standalone configuration directory | Beside the manifest |

This preserves use from `~/.agents/skills`, `~/.codex/skills`, or a collection already named `skills` without creating `skills/skills` or a nested `.claude/skills`. To distribute a collection to several agents, add `[sync]` with an explicit scope and agent selection.

## Sources and skill format

```toml
[skills]
repos = [
    "owner/single-skill-repo",                 # SKILL.md at repository root
    "owner/collection",                        # all descendant skills
    "owner/collection/skills/team",            # all skills below a collection folder
    "anthropics/skills/skills/skill-creator",    # exact subdirectory
    "anthropics/skills/skills/pdf",             # another pin from the same repo
]
```

Entries use `owner/repo` or `owner/repo/path/to/skill-or-collection`, optionally followed by `@revision`. Paths containing `.` or `..` segments are rejected. Duplicate entries are rejected; different selections from one repository are combined before reconciliation, so one selection does not remove another.

If the selected repository root or directory contains `SKILL.md`, that skill is installed. Otherwise all descendant directories containing `SKILL.md` are selected, at any depth and only inside the requested folder. Overlapping selections are deduplicated. A selection with no skills fails before target changes.

Each distinct repository/revision pair is fetched once per manifest/run and reused for all its skills and agent destinations. The checkout's Git tree is authoritative. If the API tree is stale or truncated, discovery is refreshed from the checkout before installation and version tracking.

A run installing **10 or more distinct skills** prints a warning and requires confirmation (default: no) before any target or lock is changed. Agent copies count once, and the count covers all selected manifests. A declined or unavailable prompt cancels the complete run. `--dry-run` shows the warning and plan without prompting; `--force` explicitly accepts bulk installation for CI. Up-to-date skills and updates do not count as new installations.

### Pinning versions

```toml
[skills]
repos = [
    "owner/collection/skills/pdf@v1.2.3",       # exact release tag
    "owner/another-skill@1.2.3",               # exact tag without a v prefix
    "owner/team-skills@release/stable",        # tag or branch containing a slash
    "owner/project-skills/skills/deploy@0123456789abcdef0123456789abcdef01234567", # commit
]
```

The `@revision` suffix comes after the complete source path. It accepts an exact tag, branch, or full/abbreviated commit SHA. Release versions use their exact Git tag spelling (`@v1.2.3` and `@1.2.3` are different tags). Version ranges such as `^1.2`, `~1.2` and `latest` version selection are not supported; `@latest` only works if the repository actually has a tag or branch named `latest`. Without a revision, sync follows the default branch as before.

Git fetch resolves explicit refs and the default HEAD to full commit SHAs. The checkout and lock record that commit, including when it is older than the current branch head. An unknown revision fails without falling back to the default branch. A full commit SHA is the durable pin: tags and branches can move upstream and are resolved again on each run. The lock tracks installed state, rather than freezing a moving tag. Changing a pin updates the existing installation. Different skills in a repository can use different revisions; overlapping selections of the same skill at conflicting revisions fail before installation.

The implementation follows the explicit Git-ref approach used by [Cargo](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#choice-of-commit), [npm](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/#git-urls-as-dependencies), and [pip](https://pip.pypa.io/en/stable/topics/vcs-support/#git). The compact suffix preserves existing string manifests. Exact refs avoid introducing a semver range resolver and dependency-update commands; full SHA resolution ensures discovery and checkout use the same snapshot.

Installed directories use `name` from the `SKILL.md` YAML frontmatter. Required names and descriptions are validated; scripts, references, assets, hidden supporting files and executable file permissions are copied. Git internals are excluded. Bundled symlinks and special files are rejected rather than following them outside the source tree. Two sources declaring the same installed name in one manifest are a conflict.

## Locking, conflicts and recovery

There is one consolidated lock beside each manifest, with schema version 2 and one installation record for every **source skill + absolute target directory** pair:

```toml
schema_version = 2
scope = "user"

[[installations]]
skill = "anthropics/skills/skills/skill-creator"
target = "/home/alex/.claude/skills"
directory = "skill-creator"
sha = "upstream-git-tree-sha"
content_hash = "sha256-of-installed-files"
installed_at = "2026-10-25T12:00:00Z"
updated_at = "2026-10-25T12:00:00Z"
```

The `sha` tracks the resolved commit for Git sources, or the content hash for local sources (older deployment entries may contain remote tree hashes); `content_hash` covers the installed files and executable bits. An unchanged copy is skipped. Missing copies and newly selected destinations are installed independently. Successful operations update the lock atomically; a failed target does not mark its copy installed and does not discard successful targets' state. A later run retries what is missing.

All selected manifests are planned before target changes begin. An unmanaged name collision, duplicate name within a manifest, or modified managed skill blocks the run. Local changes are preserved even with `--force`. To accept an upstream replacement, first move your edited copy to a backup location, then rerun sync; a missing managed copy can be reinstalled safely.

Removing a source or deselecting an agent proposes removal only of copies owned by that manifest's lock, including destinations no longer selected. Deletion requires confirmation or `--force`. Declining deletion preserves both the files and their lock entries. Unmanaged skills are left alone. Updates and removals keep a temporary backup until the new lock has been saved; ordinary copy/lock-write failures restore the previous installation. An interrupted process can leave a hidden `.skill-sync-*` recovery directory; it is not automatically deleted on the next run.

Schema-1 locks are imported for their original single target only. Existing legacy files must match the fetched source before a content-hash baseline can be adopted; otherwise they are preserved as an unverified conflict. A lock's user/project scope cannot be changed in place: use separate manifests and locks for those scopes. Unknown lock schemas are rejected.

Mutating CLI runs take nonblocking OS advisory locks before reading deployment state or planning changes. They lock each selected manifest's state file, all selected target directories, and recorded targets that may need cleanup. Canonical paths deduplicate symlink aliases. Another run using the same resource fails before target changes; unrelated resources can sync independently.

Coordination files live under `~/.cache/skill-sync/run-locks`, independently of `XDG_CONFIG_HOME`, and stay in place after exit. The OS releases held locks when the process exits or crashes. Do not delete coordination files while syncs are running: replacing the file would allow different processes to lock different files. These advisory locks coordinate skill-sync CLI processes using the same home directory; other programs and direct library callers must coordinate their own writes. Filesystems must support OS file locks.

Status is a read-only snapshot and does not create coordination files or acquire locks. Dry-run never writes manifests, deployment locks or destinations, but remote resolution uses the shared repository cache and its coordination lock. A concurrent writer can change the files they inspect; retry the inspection after the writer completes. A mutating sync repeats its own planning while holding locks.

## CLI reference

| Flag | Behavior |
|---|---|
| `status` / `doctor` | Inspect local files and recorded installations without network requests or writes |
| `--json` | Emit one schema-versioned JSON result on stdout, including errors and per-operation outcomes |
| `--init` | Create `.skills.toml` in the working directory, or at `--config` |
| `--global --init` | Create the central config with Codex and Claude Code selected |
| `--global` | Process only the central user config; force user scope |
| `--local` | Process only the nearest project manifest; force project scope |
| `--config PATH` | Process exactly this manifest; honor its explicit scope |
| `--agent ID` | Override agent selection; repeat or comma-separate IDs |
| `--target PATH` | Replace the destination list with one explicit directory; conflicts with `--global` |
| `--dry-run` | Resolve sources and print the plan; change no target/config/lock files |
| `--force` | Confirm bulk installations and managed removals; never overwrite local edits or unmanaged skills |
| `--verbose` | Print manifest, lock, scope and resolved target paths |

Dry-run may fetch remote sources into the persistent repository cache to validate the real skill names and files. It does not change destinations, manifests or deployment locks. Remote previews may populate or evict the repository cache and acquire its coordination lock.

Exit codes:

| Code | Meaning |
|---|---|
| `0` | Successful sync/init, no pending dry-run changes, or locally healthy status |
| `1` | Configuration, source, filesystem, concurrency or sync failure |
| `2` | Pending dry-run changes, local status needing attention, or a declined managed removal; also CLI argument errors |

In JSON output, use `success`, `command`, `errors` and `pending_changes` to distinguish a failed invocation from a successful inspection with findings. A declined bulk-install confirmation cancels the run with code `1`. `status` accepts scope, manifest, agent, target, verbose and JSON options, before or after the command. It rejects `--init`, `--force` and `--dry-run`.

## Offline status

```sh
skill-sync status
skill-sync status --local --json
skill-sync doctor --global
```

Status reads the same selected manifests and deployment locks as sync, rebases project paths in memory, and hashes local files. It never discovers credentials, launches Git/gh, fetches sources, adopts legacy baselines or writes state. It reports:

| State | Meaning |
|---|---|
| `unchanged` | Managed files match their recorded content hash |
| `modified` | Managed files differ from that hash |
| `missing` | A recorded copy, or a known skill at a newly selected target, is absent |
| `unverified` | A legacy installation has no recorded content hash |
| `unmanaged` | A skill exists without ownership by this manifest |
| `unreadable` | Files cannot be inspected, including replaced/symlinked managed directories |

Entries include `managed` and `selected`. Managed entries no longer selected need reconciliation. Unrelated unmanaged skills are informational; an unmanaged name occupying a known, selected skill's destination needs attention. A source with no matching recorded skill appears under `unresolved_sources`: sync is required to discover its names and files. Unreadable targets also need attention.

`upstream_checked` is always `false`. A clean local status means files match recorded installations. It does not verify a changed revision suffix, current branch/tag content, new/deleted members of a collection, or overlapping remote selections. Use `--dry-run` for an upstream-aware plan. Unmanaged discovery lists direct children containing `SKILL.md`; it is not an inventory of every agent's discovery rules.

## JSON contract

`--json` works with sync, dry-run, status and init. Stdout contains exactly one JSON object. Diagnostics and any confirmation prompts use stderr. Pass `--force` when intentionally accepting bulk installation or managed removal in unattended syncs. Help and version requests still print their normal text.

The top-level `schema_version` is `1`, separate from the deployment-lock schema. Common fields:

| Field | Meaning |
|---|---|
| `command` | `sync`, `status`, `init`, or `arguments` for parsing errors |
| `dry_run` | Whether this was a sync preview |
| `success` | Whether execution/inspection completed without an error |
| `pending_changes` | Planned or unapplied changes, or local status findings |
| `exit_code` | The process exit code |
| `manifests` | Selected manifests that reached inspection/planning |
| `errors` | Runtime or argument errors |
| `created` | Manifest path for successful init, otherwise null |

Each manifest contains `manifest`, `lock`, `scope`, `targets`, `unchanged`, `conflicts`, `operations` and `status`. `status` is null for sync; status inspections have no planned operations. Each operation includes `action` (`install`, `update`, `remove`, `forget`), `skill`, `target`, `outcome` and an optional `error`.

Outcomes are `planned`, `applied`, `skipped` or `failed`. A failed sync may contain applied operations: successful targets retain their recorded state. Planned outcomes may remain when a conflict or confirmation stops execution. An error before source preparation completes may leave `manifests` empty. Warnings remain human-readable on stderr.

```sh
skill-sync --local --dry-run --json > plan.json
# Code 2 with success=true means pending changes, not a failed plan.
skill-sync status --local --json > status.json
```

## Authentication

Uses `GITHUB_TOKEN`, then `gh auth token`, then unauthenticated GitHub access. Empty manifests require no GitHub requests or checkout. Private repositories need authentication; GitHub API rate limits apply. Sync never executes bundled skill scripts.

## Examples and development

See [user.skills.toml](../examples/user.skills.toml), [project.skills.toml](../examples/project.skills.toml), and the legacy [minimal](../examples/minimal.skills.toml), [monorepo](../examples/monorepo.skills.toml) and [mixed](../examples/mixed.skills.toml) manifests.

```sh
cargo fmt --all --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
sh -n install.sh
python3 -m unittest discover -s tests -p 'test_install.py' -v
./scripts/test-sync.sh         # networked smoke test in a temporary project
./scripts/test-multi-agent.sh  # networked user + project + multi-target test
```

Automated tests cover independent copies, shared target deduplication, central/project discovery, custom paths, per-target retries, deletion, local edits, rollback after lock failures, old locks and rebasing project locks. Source tests use local Git repositories and a local API fixture; they verify recursive collections, historical commit and tag pins, conflicting revisions, bulk-install confirmation, and reuse of a checkout for multiple selections at one revision. Tests verify the portable filesystem layout without launching live AI agent sessions.


PR CI runs the Rust suite on macOS/Linux stable and Linux Rust 1.89, plus installer tests. Formatting and Clippy run on Linux. Networked public-GitHub smoke tests remain optional; the automated source tests use local Git repositories and local HTTP fixtures. Runtime dependencies are Git, remote repository access for uncached Git sources, and a filesystem with OS advisory-lock support. Building requires Rust 1.89 or newer. Offline status needs none of the network or Git dependencies.


## Pre-v2 source management

These commands are implemented; public-index discovery remains outside this release.

```sh
skill-sync add 'local:../private-skills' --local --dry-run
skill-sync add 'local:../private-skills' --local
skill-sync remove 'local:../private-skills' --local
skill-sync add 'git+ssh://git@git.example.com/team/skills.git//skills/review#v1' --local
skill-sync --local --diff
skill-sync lock --local
skill-sync --local --frozen --dry-run
skill-sync --local --frozen
skill-sync update --local --dry-run
skill-sync update --local
```

`add` and `remove` edit exactly one existing manifest, preserving comments and unrelated tables. Select it with `--local`, `--global` or `--config` if discovery finds both scopes. `--agent` explicitly replaces that manifest's agent selection. An identical addition or absent removal is a no-op. Preview returns 2 when an edit is pending; applying an edit returns 0 and does not install or remove copies. Initialization remains a separate `--init` command. Source duplicates use normalized exact selections, including their refs; conflicting overlapping selections fail during sync. External edits between reading and saving are rejected.

Sources accept the existing GitHub shorthand, `local:PATH`, and `git+https://HOST/PATH.git[//SKILL-PATH][#REF]` or `git+ssh://USER@HOST/PATH.git[//SKILL-PATH][#REF]`. The double slash after the repository selects a skill or descendant collection; `#` selects an exact ref for URL sources. GitHub shorthand continues to use `@`. HTTPS and SSH are the remote transport allowlist; HTTP, file, ext transports, URL passwords and embedded HTTPS usernames are rejected. Local paths resolve relative to the manifest, including `..` and absolute paths. Relative local paths are the portable option. Sources may live outside a project; project destinations still cannot escape it. Missing sources fail preparation before target mutation. A selected root containing SKILL.md installs that skill. Otherwise the shared resolver discovers SKILL.md files at every descendant depth, preserving the existing collection rules. Symlinks and special resources are rejected. All installations are independent snapshots with content hashes; local changes are never propagated back to the source.

For additional tools, choose explicit paths with `custom` or distinct `custom:NAME` agent IDs:

```toml
[sync]
scope = "project"
agents = ["codex", "custom:editor", "custom:reviewer"]

[agents."custom:editor"]
project_dir = ".editor/skills"
user_dir = "~/.editor/skills"

[agents."custom:reviewer"]
project_dir = ".reviewer/skills"

[skills]
repos = ["local:../team-skills"]
```

Custom IDs have no guessed defaults: provide the selected scope's path. Unknown native IDs remain errors. Canonical path deduplication, ownership, conflict detection and writer locks apply equally to custom targets.

### Portable source locks

`lock` creates `.skills.sources.lock` (or `skills.sources.lock`) beside the manifest. An existing source lock is verified and reused; `update` explicitly resolves current revisions and replaces it. Neither command installs anything. Both support JSON and `--dry-run`; their preview exits 2 when the lock would change. Commit this source lock with the manifest for teammates and CI. Keep `.skills.lock` as the machine's installation ledger. The source lock stores schema version 1, the exact source selections, immutable Git commits, selected relative skill paths, names and file hashes. It contains no target paths or credentials. Absolute local sources remain machine-specific by explicit choice.

`--frozen` requires a matching source lock and verifies every selected skill's content and collection membership. Moving remote refs are ignored in favor of the recorded commits. Local source edits, ref edits, additions, removals, changed skill names and mismatched hashes fail before deployment. Frozen installation uses cached objects offline; missing cached objects require access to fetch the exact locked commit. A missing or corrupt source lock fails with a request to review and regenerate it. Ordinary sync continues to follow moving refs and does not silently rewrite a portable lock.

### Cache and authentication

Bare Git repositories live under `~/.cache/skill-sync/repos`, keyed by repository URL, with immutable commit refs retained for frozen use. A run reuses checkouts for identical repository/ref selections. Normal sync fetches the requested ref again to see upstream changes; frozen sync uses cached commits without fetching when available. Repository integrity checks repair corrupted caches by recreating them and fetching again. Git fetches use the installed credential helpers or SSH agent; GitHub sources additionally honor `GITHUB_TOKEN`, then `gh auth token`. Tokens are passed through a child-only environment header, never stored in URLs, cache configuration or source locks. Git diagnostics intentionally avoid echoing authentication details. SSH host setup remains the user's Git configuration.

`--cache-limit-mib N` caps retained repository data after preparation (default 512 MiB); least recently used repositories are evicted first. Zero removes all cached repositories after staging. The limit covers retained bare repositories; temporary checkouts and in-progress fetches can use additional disk. Eviction may require a later frozen installation to fetch its commit again. Cache readers, repair and eviction share a nonblocking advisory lock; a concurrent cache user causes a retryable error. Offline status never touches this cache.

### File differences and baselines

`--diff` implies dry-run. JSON adds `diffs`; text prints file patches before the operation plan. For each changed installation, `upstream` compares its installed baseline with proposed source files; `local` compares that baseline with the current destination. Before the first baseline is retained, `installed_to_proposed` labels the available two-way comparison. Changed paths include additions, removals, binary summaries and executable-bit differences. Text patches show whole files, with control characters escaped and large content summarized. Patches are bounded previews, not an automatic merge or patch-rebasing mechanism. Conflicts still exit 1 with local files preserved.

Successful installs retain comparison copies under `.skills.baselines/` (or `skills.baselines/`) beside the deployment ledger, keyed by content hash. Do not commit these directories. They contain copies of installed resources, including any private material. Removing them is safe when no sync is running; future previews use the two-way comparison until another installation retains a baseline. JSON also adds `manifest_edits` (path/before/after) and `source_locks` (paths) and `source_resolutions` (resolved source-lock contents) to the version-1 report contract.

## Release validation

PR CI validates macOS ARM64, Linux x86_64 and the minimum Rust toolchain. The release workflow calls the same CI before building, packages both platforms, verifies checksums through the actual installer, and runs personal/project initialization, local-source selection, preview, two-agent installation, idempotence, frozen installation, edit protection and backup/recovery against each packaged binary. Publication waits for all packages and smoke tests, and requires a tag matching Cargo.toml. Distribution-related pull requests and manual dispatch validate packages without publishing. HTTPS uses Rustls so the Linux musl binary does not require system OpenSSL.

A production release and a smoke test of the publicly downloaded assets remain separate from local implementation checks; no release is published just by building this checkout.
