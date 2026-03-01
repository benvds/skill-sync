# Milestones

## v1.0 MVP (Shipped: 2026-03-01)

**Delivered:** Declarative skill management CLI — commit `.skills.toml`, run `npx skills-sync`, every teammate has the same skills.

**Stats:** 3 phases, 10 plans | 68 commits | 2,703 LOC TypeScript | 2 days
**Git range:** `feat(01-01)` → `feat(03-02)`

**Key accomplishments:**
1. Declarative TOML manifest parsing with `--init` scaffold, config discovery, and line/column error reporting
2. Global and local state reading from Vercel's lock file and filesystem scanning
3. Pure diff computation with install/remove/unchanged categorization
4. Orphan detection and removal with interactive prompts, TTY guard, and `--yes` CI bypass
5. `--check` CI gate enforcement (exit code 0/1 for sync status)
6. npm package distribution ready via `npx skills-sync`

### Tech Debt (7 non-critical items)
- Dead code: `SkillsUnavailableError`, `reporter.warnLock*` methods never called
- `SyncResult.removed` hardcoded to 0 in `executeSync`
- `--dry-run` doesn't bypass orphan prompt in TTY; misreports orphan count in non-TTY
- Init file-already-exists error uses generic hint

**Archive:** `.planning/milestones/v1.0-ROADMAP.md`, `.planning/milestones/v1.0-REQUIREMENTS.md`

---

