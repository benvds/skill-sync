# skill-sync product requirements

Status: proposed product direction, replacing the original PRD. Updated: 2026-10-25.

This document separates implemented behavior from future requirements. Version numbers below are product milestones; the current Cargo package is `0.1.0`.

## Product decision

Build a small tool that lets developers declare which skills their AI agents should have, inspect the installed files, and apply changes without silently overwriting local work.

Grow in stages: reliable terminal management in v1, public-index discovery in v2, and a desktop interface built with egui in v3. Each stage uses the same installation rules and state.

Implementation details: [CLI and configuration reference](reference.md). Delivery sequence: [roadmap](ROADMAP.md).

## Problem and audience

A developer using several agents manages several skill directories. Personal preferences and project requirements have different lifetimes. Upstream changes, edited copies and forgotten installations make manual copying hard to audit.

The primary user is a developer who uses one or more supported agents and can edit a small configuration file. A second user is a team member setting up project skills from a committed manifest. Automation consumes plans and outcomes through JSON.

The user should be able to answer:

- Which sources and agents did I choose for myself and this project?
- Which local copies are missing or changed?
- What will sync install, update or remove?
- What succeeded, what failed, and what can I retry?

See [offline status](reference.md#offline-status) for the distinction between local health and remote freshness.

## Principles and boundaries

The manifest expresses intent. Each deployment lock records ownership and installed state. The filesystem is checked before managed files are replaced or removed.

Independent copies are the default and the currently supported mode. Changes in one agent's directory must not modify another installation or the source. Personal and project scopes remain separate; settings are not silently merged.

An error in configuration, source preparation or conflict checks stops target changes. Applying a valid plan can succeed at some destinations and fail at others. Successful operations keep their state; failed operations preserve or restore previous files where recovery is possible. A run is not a transaction across all targets.

The product manages skill files. It does not execute their scripts, guarantee prompt safety, decide an agent's discovery precedence, or upload files to cloud assistants. MCP, hooks, agent definitions and a hosted marketplace are outside these milestones.

See [locking and recovery](reference.md#locking-conflicts-and-recovery) for current guarantees and limitations.

## Version 1: reliable terminal management

### Scope

V1 keeps the existing product shape: edit a manifest, preview changes, sync through the terminal, and inspect local installations. Simple installation choices make the CLI accessible without requiring a development toolchain.

| ID | Requirement | Implementation status in this change |
|---|---|---|
| V1-01 | Central personal manifest and nearest project manifest, with explicit scope/config selection | Existing |
| V1-02 | Explicit agent targets, native paths and custom destinations; canonical aliases are deduplicated | Existing |
| V1-03 | GitHub repositories, exact paths, recursive collections, exact Git refs and full SHA pins | Existing |
| V1-04 | Independent copies and per-skill/per-target ownership and content hashes | Existing |
| V1-05 | Preview and conflict checks before mutation; edited/unmanaged files are preserved | Existing |
| V1-06 | Confirm bulk installs and owned removals; support unattended acceptance without bypassing edit protection | Existing |
| V1-07 | Offline local status, with a clear statement that upstream was not checked | Implemented |
| V1-08 | Versioned JSON for plans, outcomes, status, init and errors | Implemented |
| V1-09 | Prevent concurrent CLI writes to the same manifest or target, including deselected targets | Implemented |
| V1-10 | Recoverable writes and independently retryable target failures | Existing |
| V1-11 | Binary installer with version/directory options, checksum verification and atomic replacement | Implemented; requires a new release containing checksum sidecars |
| V1-12 | CI on supported OS families, minimum Rust, formatting, Clippy and installer regression tests | Workflow added; hosted CI must pass before release |

V1 distribution targets are macOS ARM64 and Linux x86_64. Cargo source installation requires Rust 1.89 or newer. Additional package managers and architectures are optional after their install paths are tested.

### Acceptance criteria

1. A user can declare one source and two agents, preview, install and rerun without rewriting unchanged copies.
2. Personal sync does not copy personal skills into project repositories; project destinations cannot escape the project boundary.
3. `status` launches no Git/gh processes, makes no network calls, creates no directories or files, and never changes baseline hashes.
4. Status distinguishes unchanged, modified, missing, unverified, unmanaged and unreadable local entries. It reports unresolved sources and previously owned destinations no longer selected. It labels remote freshness as unchecked.
5. JSON stdout contains one parseable object. Human diagnostics and prompts stay on stderr. Results distinguish planned, applied, skipped and failed operations, including partial success.
6. JSON and text commands agree on exit codes: 0 for completion/local health; 1 for runtime failure; 2 for successful inspection with findings or declined removals. Argument errors also use 2 and are identified separately in JSON.
7. Two CLI processes using the same home cannot mutate the same canonical manifest or target concurrently. Independent resources can run independently. Killing a holder allows a later run without deleting coordination files.
8. Missing or incorrect release checksums and download failures preserve an installed binary. Destination paths containing spaces work. Version selection avoids querying the latest-release endpoint.
9. CI covers filesystem behavior, scope discovery, historical source pins, partial failure, rollback, concurrency and installer failure paths. Both supported OS families pass before release.

### Explicit limits

The deployment lock is an installation ledger, not a dependency-resolution lock. Moving tags and branches are resolved on every sync. Full SHA suffixes provide source pins today. Status cannot validate changed manifest refs or remote collection membership from local state alone.

Local edits are detected and preserved by stopping replacement; V1 does not merge them. Sources remain GitHub-only. Copy mode, selected native targets and compatibility behavior remain supported. Read-only inspections are snapshots and may race a concurrent writer; advisory coordination applies to CLI writers using the same home, not arbitrary filesystem tools.

Candidate follow-ups include local sources, manifest add/remove helpers, update diffs, caching and a portable source lock with frozen installation. These require separate acceptance criteria and are not silently added to the V1 release gate.

Operational contract: [reference](reference.md). Follow-up priorities: [roadmap](ROADMAP.md#after-the-v1-baseline).

## Version 2: discover skills from public indexes

### User outcome

A user can search a public index, inspect a skill's source and contents, choose a scope and agents, and add that exact source selection to a manifest. Installation uses the existing preview and sync engine.

Start with [skills.sh](https://www.skills.sh/). Keep the provider boundary small so another public index can be added without changing installation semantics. The availability and terms of a supported search API must be confirmed before committing to an integration; the public website is not an API contract.

### Requirements

| ID | Requirement |
|---|---|
| V2-01 | Terminal search/browse with query, bounded results, pagination and provider/source attribution |
| V2-02 | Detail view showing name, description, canonical repository/path, source link and any available revision information |
| V2-03 | Explicit selection of scope, agents and optional ref before adding a source |
| V2-04 | Manifest edits preserve existing comments and unrelated configuration; duplicate sources are handled predictably |
| V2-05 | A preview separates manifest edits from installation changes; remote content is never installed just by browsing |
| V2-06 | Network failures, provider rate limits, unsupported source types and stale entries have actionable outcomes |
| V2-07 | JSON search/detail results and fixture-based provider tests; normal sync remains independent of index availability |
| V2-08 | Bound response sizes, escape terminal control characters in untrusted metadata, and validate source coordinates through the existing resolver |

Possible command names are `search`, `show` and `add`. They are proposals, not current CLI commands. A terminal browser may begin as paginated text and selection prompts; a full-screen TUI is optional.

### Release gate

A new user finds a skill through the first provider, inspects its canonical source, adds it to a chosen manifest and installs it through a reviewable plan. The flow works for a single-skill repository and a collection entry without adding unrelated skills. Unsupported non-GitHub results are explained rather than rewritten to a guessed source.

Provider outages do not prevent syncing existing manifests. Tests use captured or authored provider fixtures and cover pagination, malformed metadata and source mapping. Popularity and index audit labels are displayed with provenance; they are not treated as a safety guarantee.

Provider discovery: [skills.sh](https://www.skills.sh/). Planned sequence: [V2 roadmap](ROADMAP.md#v2-public-index-browser).

## Version 3: egui desktop app

### User outcome

A user can manage personal and project skills, browse the V2 index, review changes and sync from a desktop window. The terminal workflow remains supported.

### Requirements

| ID | Requirement |
|---|---|
| V3-01 | Separate egui application, with eframe or an explicitly chosen native integration |
| V3-02 | Personal/project selection, installed-state list, scope/agent/source details and local-status findings |
| V3-03 | Search and detail views using the V2 provider interfaces |
| V3-04 | Reviewable installation/removal plans, explicit confirmation and progress/results per destination |
| V3-05 | Shared Rust application services for discovery, planning, locking and application; no second implementation of file ownership rules |
| V3-06 | Network, Git and filesystem work runs outside the UI thread; progress is delivered as events |
| V3-07 | GUI and CLI coordinate through the same locks, manifests and deployment state |
| V3-08 | CLI installations do not require GUI dependencies; desktop builds, packaging and update behavior are explicit |
| V3-09 | Keyboard navigation, readable status/error text and supported-display-scale checks |

### Release gate

The same fixture produces the same plan and filesystem outcome through CLI and GUI services. A GUI sync and CLI sync cannot write the same resources concurrently. Users can inspect failure details and retry without losing successful targets. An index or network failure leaves the window responsive.

Initial desktop targets follow the supported CLI families, macOS and Linux. Windows support is a separate decision requiring installer, path, permissions and locking validation. Web/wasm deployment, background daemons and cloud accounts are outside V3.

The egui version and minimum Rust requirement must be selected when implementation starts; they are not fixed by this planning document. See [egui's official documentation](https://docs.rs/egui/latest/egui/) and the [V3 roadmap](ROADMAP.md#v3-egui-desktop-app).

## Release evidence and remaining decisions

Release readiness is demonstrated by a fresh-install walkthrough, a project walkthrough, automated regression results on supported platforms, and a documented recovery example. These are release checks, not claims about adoption or measured user outcomes.

Before V2 implementation, decide the supported provider access method and caching terms. Before V3 implementation, decide desktop packaging and extract shared application services from terminal-specific prompting/output. Frozen locks and general Git transports remain separately scoped follow-ups; no milestone implies they already exist.

## Sources

- [Current behavioral reference](reference.md): configuration, locking, status, JSON and recovery contracts.
- [Roadmap](ROADMAP.md): milestones and dependencies.
- [Agent Skills specification](https://agentskills.io/specification): the skill format this tool installs.
- [skills.sh](https://www.skills.sh/): the first proposed public discovery index.
- [egui documentation](https://docs.rs/egui/latest/egui/): the requested desktop UI technology.
