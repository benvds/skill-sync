# skill-sync roadmap

Updated: 2026-10-25. Product milestones describe delivery order, not promised dates. The current package version is `1.0.0`.

Build a dependable terminal workflow first. Add public-index discovery to that workflow, then expose the same behavior in an egui desktop app.

The [PRD](PRD.md) defines acceptance criteria. The [reference](reference.md) describes implemented commands. Commands and capabilities marked planned below are not implemented by this change.

## V1: terminal sync

Goal: a developer can install skill-sync, declare personal/project skills, inspect local state and apply a reviewed plan.

| Milestone | Scope | State |
|---|---|---|
| 1A. Declarative installation | Scopes, agent destinations, copies, recursive GitHub collections, exact refs, ownership and recovery | Existing baseline |
| 1B. Inspection and automation | Offline `status`/`doctor`, JSON plans/results/errors, accurate partial outcomes | Implemented in this change |
| 1C. Writer coordination | Manifest and current/previous target locks; canonical paths; automatic release after crash | Implemented in this change |
| 1D. Distribution and checks | Version/directory installer options, checksum sidecars, atomic replacement, PR CI, updated docs | Implemented; release gated on reusable CI and packaged onboarding smoke checks |
| 1E. V1 release validation | Run supported-platform CI, publish checksummed assets, test the published installer and documented onboarding | Release checks automated; publishing and public-download verification remain separate |

Keep V1 close to the present scope. A binary installer and Cargo are enough initial installation routes. Homebrew, more architectures and shell completions are optional improvements once supported and tested.

Exit V1 when the PRD's V1 acceptance criteria pass on macOS and Linux and the published artifacts pass an installation smoke test. Publishing a release remains a separate action.

Further reading: [V1 requirements](PRD.md#version-1-reliable-terminal-management), [current CLI](reference.md#cli-reference).

## After the V1 baseline

All six follow-ups below are now implemented. They extend terminal management without adding V2 index discovery.

| Order | Delivered feature | User value | Implemented contract |
|---|---|---|---|
| 1 | Local directory sources | Develop and share private skills without pushing every edit | `local:` paths relative to the manifest; staged copies, hashes and missing-source errors |
| 2 | Comment-preserving manifest add/remove helpers | Change configuration from the terminal; prepare for V2 selection | `add`/`remove`, idempotent selections, comment-preserving TOML and preview |
| 3 | File diff before replacement | Understand upstream and local changes | `--diff`, retained content baselines, separate upstream/local changes and bounded binary summaries |
| 4 | Persistent repository cache | Reduce repeated downloads and make remote previews faster | Immutable commit objects, live-ref refresh, integrity repair, LRU retention and cache writer coordination |
| 5 | Portable source lock and frozen installation | Restore reviewed source content in teammates' checkouts and CI | `lock`, `update` and `--frozen`; portable commit/content lock separate from the deployment ledger |
| 6 | More Git transports and targets | Use internal repositories and additional tools | HTTPS/SSH allowlist, Git credential helpers/SSH agent and named custom agent paths |

Full SHA suffixes and portable source locks provide source pins. Local edit protection remains conflict detection; automatic patch rebasing is a separate decision.

Further reading: [current source behavior](reference.md#sources-and-skill-format), [current state locking](reference.md#locking-conflicts-and-recovery).

## V2: public-index browser

Goal: find a skill, inspect it and add its canonical source without copying repository coordinates by hand.

| Sequence | Deliverable | Dependency / completion evidence |
|---|---|---|
| 2A | Confirm skills.sh access method and terms; define provider result/detail types | A supported integration path and representative fixtures; no assumed stable endpoint |
| 2B | CLI search and detail view, pagination, source links and JSON | Tests for results, empty pages, malformed data, rate limits and outages |
| 2C | Select a result, scope, agents and optional ref; preview the manifest edit | Comment-preserving TOML edits and validated canonical source mapping |
| 2D | Add to manifest and use the existing plan/sync flow | End-to-end single-skill and collection cases; no installation during browsing |
| 2E | Harden provider boundary and document supported sources | Bounded inputs, terminal-safe metadata, outage isolation and actionable unsupported-source errors |

Start with [skills.sh](https://www.skills.sh/), then consider another index only when the first integration exposes a concrete need. Reuse the existing resolver. Public results that cannot be installed by supported source adapters remain visible with an explanation.

A paginated terminal browser is sufficient. A full-screen TUI, ranking engine, proprietary index and account system are outside the initial V2 gate.

Exit V2 when a user can search, inspect, add and sync through one documented terminal workflow, and existing-manifest sync works during an index outage.

Further reading: [V2 requirements](PRD.md#version-2-discover-skills-from-public-indexes), [skills.sh](https://www.skills.sh/).

## V3: egui desktop app

Goal: offer the V1 management workflow and V2 discovery in a native desktop window.

| Sequence | Deliverable | Dependency / completion evidence |
|---|---|---|
| 3A | Extract shared application services and explicit progress/confirmation events | CLI behavior is preserved; terminal prompting is outside core services |
| 3B | Separate egui application with personal/project selection and local status | No GUI dependencies in the default CLI build; responsive background work |
| 3C | Search, skill details and manifest-selection UI | Uses the V2 providers and source validation |
| 3D | Review plan, confirm, apply, show results and retry | Same fixture plans/outcomes as CLI; shared locks block simultaneous writers |
| 3E | Desktop packaging and usability pass | macOS/Linux installation, keyboard navigation, display scaling and error/recovery walkthroughs |

Use Rust APIs directly for shared behavior. JSON remains the external automation contract; it is not required as an internal GUI transport. Git and blocking HTTP calls run on workers, with progress delivered to the egui event loop.

Choose an egui/eframe version when work starts and document its Rust requirement. Review Windows separately after the supported platforms work; do not infer support from egui portability alone.

Exit V3 when a user completes onboarding, discovery, review and sync in the app, while the CLI stays independently installable and both frontends honor the same ownership and recovery rules.

Further reading: [V3 requirements](PRD.md#version-3-egui-desktop-app), [official egui documentation](https://docs.rs/egui/latest/egui/).

## Deferred across these milestones

A hosted marketplace, cloud sync/accounts, execution sandbox, MCP/hook/plugin management and automatic local-patch rebasing are outside the agreed direction. Add them only through an explicit scope change.

Keep one manifest model, one ownership model and one installation engine as the product grows. See [PRD boundaries](PRD.md#principles-and-boundaries) for the guarantees each frontend must preserve.
