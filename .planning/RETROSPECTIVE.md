# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-01
**Phases:** 3 | **Plans:** 10 | **Sessions:** ~3

### What Was Built
- Declarative TOML manifest parsing with `--init`, config discovery, line/column errors
- Global state reading (Vercel lock file) and local state reading (filesystem + `.skills-sync-sources.json`)
- Pure diff computation with orphan detection and interactive removal
- Full CLI with `--global`, `--local`, `--dry-run`, `--yes`, `--verbose`, `--agent`, `--config`, `--check`
- CI gate via `--check` (exit code 0/1)
- npm package distribution via `npx skills-sync`

### What Worked
- TDD commit sequence (RED tests → GREEN implementation) kept each plan focused and verifiable
- Thin wrapper strategy: zero GitHub/git code written, all delegated to `npx skills`
- Phase-based decomposition created clear boundaries (types → parsing → state → CLI → extensions → distribution)
- ESM-only choice avoided CJS compatibility overhead entirely
- 156 tests passing at ship gave confidence for rapid iteration

### What Was Inefficient
- Phase 1 plan checkboxes in ROADMAP.md were never updated to `[x]` (Phase 2/3 were)
- Some dead code shipped (SkillsUnavailableError, reporter.warnLock* methods) — defined speculatively in early plans, never wired
- Phase 3 ROADMAP shows "Plans: TBD" and "0/TBD" despite being complete — plan count wasn't updated after planning

### Patterns Established
- Graceful null return pattern for state readers (not throws) — consistent fallback behavior
- `__local__` sentinel key as guard for orphan detection availability
- Single dry-run boundary in executor (not threaded through modules)
- Verbose gating pattern: `if (options.verbose)` before printing commands
- Atomic write via tmp+rename for `.skills-sync-sources.json`

### Key Lessons
1. Defining typed errors upfront is good, but defer implementation until the throw site is written — avoids dead code
2. The thin wrapper over `npx skills` validated the strategic pivot — the manifest format is the core IP, not the plumbing
3. 2,703 LOC for a full CLI with 10 flags, orphan detection, CI gate, and npm distribution confirms TypeScript was the right call over Rust

### Cost Observations
- Model mix: balanced profile (sonnet-dominant with opus for planning)
- Sessions: ~3 sessions over 2 days
- Notable: 10 plans averaged ~2.75 min each — very fast execution

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | ~3 | 3 | Initial project — TDD, thin wrapper, ESM-only |

### Cumulative Quality

| Milestone | Tests | Coverage | Tech Debt Items |
|-----------|-------|----------|-----------------|
| v1.0 | 156 | — | 7 (all non-critical) |

### Top Lessons (Verified Across Milestones)

1. Delegate to ecosystem tools rather than reimplementing — saves massive effort
2. TDD commit sequence keeps plans focused and produces verifiable deliverables
