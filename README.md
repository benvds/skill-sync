# skill-sync

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

Explicit revisions are resolved through GitHub to a full commit SHA before fetching. The checkout and lock record that commit, including when it is older than the current branch head. An unknown revision fails without falling back to the default branch. A full commit SHA is the durable pin: tags and branches can move upstream and are resolved again on each run. The lock tracks installed state, rather than freezing a moving tag. Changing a pin updates the existing installation. Different skills in a repository can use different revisions; overlapping selections of the same skill at conflicting revisions fail before installation.

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

The `sha` tracks the resolved commit for explicit revision pins, or the remote tree for sources following the default branch; `content_hash` covers the installed files and executable bits. An unchanged copy is skipped. Missing copies and newly selected destinations are installed independently. Successful operations update the lock atomically; a failed target does not mark its copy installed and does not discard successful targets' state. A later run retries what is missing.

All selected manifests are planned before target changes begin. An unmanaged name collision, duplicate name within a manifest, or modified managed skill blocks the run. Local changes are preserved even with `--force`. To accept an upstream replacement, first move your edited copy to a backup location, then rerun sync; a missing managed copy can be reinstalled safely.

Removing a source or deselecting an agent proposes removal only of copies owned by that manifest's lock, including destinations no longer selected. Deletion requires confirmation or `--force`. Declining deletion preserves both the files and their lock entries. Unmanaged skills are left alone. Updates and removals keep a temporary backup until the new lock has been saved; ordinary copy/lock-write failures restore the previous installation. An interrupted process can leave a hidden `.skill-sync-*` recovery directory; it is not automatically deleted on the next run.

Schema-1 locks are imported for their original single target only. Existing legacy files must match the fetched source before a content-hash baseline can be adopted; otherwise they are preserved as an unverified conflict. A lock's user/project scope cannot be changed in place: use separate manifests and locks for those scopes. Unknown lock schemas are rejected.

Do not run concurrent synchronizations against the same manifest: lock files track installed state, and do not currently provide a process mutex.

## CLI reference

| Flag | Behavior |
|---|---|
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

Dry-run may fetch remote sources into temporary storage to validate the real skill names and files. Exit codes: `0` for success/up to date, `2` for a dry-run with pending changes, `1` for sync/configuration conflicts or errors. CLI argument errors also use exit code `2`.

## Authentication

Uses `GITHUB_TOKEN`, then `gh auth token`, then unauthenticated GitHub access. Empty manifests require no GitHub requests or checkout. Private repositories need authentication; GitHub API rate limits apply. Sync never executes bundled skill scripts.

## Examples and development

See [user.skills.toml](examples/user.skills.toml), [project.skills.toml](examples/project.skills.toml), and the legacy [minimal](examples/minimal.skills.toml), [monorepo](examples/monorepo.skills.toml) and [mixed](examples/mixed.skills.toml) manifests.

```sh
cargo test
cargo clippy --all-targets
./scripts/test-sync.sh         # networked smoke test in a temporary project
./scripts/test-multi-agent.sh  # networked user + project + multi-target test
```

Automated tests cover independent copies, shared target deduplication, central/project discovery, custom paths, per-target retries, deletion, local edits, rollback after lock failures, old locks and rebasing project locks. Source tests use local Git repositories and a local API fixture; they verify recursive collections, historical commit and tag pins, conflicting revisions, bulk-install confirmation, and reuse of a checkout for multiple selections at one revision. Tests verify the portable filesystem layout without launching live AI agent sessions.
