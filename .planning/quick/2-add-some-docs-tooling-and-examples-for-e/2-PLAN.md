---
phase: quick-2
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - README.md
  - examples/minimal.skills.toml
  - examples/monorepo.skills.toml
  - examples/mixed.skills.toml
  - scripts/test-sync.sh
autonomous: true
requirements: []

must_haves:
  truths:
    - "New users can understand what skills-sync does and how to install/use it from the README"
    - "Users have example manifests covering the three repo layouts (single-skill, monorepo, subdirectory pin)"
    - "A developer can run one script to do a quick manual integration test with real repos"
  artifacts:
    - path: "README.md"
      provides: "Project documentation with install, usage, manifest format, CLI reference"
      min_lines: 80
    - path: "examples/minimal.skills.toml"
      provides: "Single-skill repo example manifest"
    - path: "examples/monorepo.skills.toml"
      provides: "Monorepo auto-detect example manifest"
    - path: "examples/mixed.skills.toml"
      provides: "Mixed layout example with subdirectory pinning"
    - path: "scripts/test-sync.sh"
      provides: "Manual integration test script exercising init, dry-run, sync, and verbose"
  key_links:
    - from: "README.md"
      to: "examples/"
      via: "Markdown links to example manifests"
      pattern: "examples/"
    - from: "scripts/test-sync.sh"
      to: "cargo run"
      via: "Runs skills-sync via cargo with test manifests"
      pattern: "cargo run"
---

<objective>
Add project documentation, example manifests, and a manual integration test script so that new users can get started quickly and developers can verify the tool works end-to-end without reading source code.

Purpose: The project is feature-complete but has zero user-facing documentation, no examples, and no convenient way to manually smoke-test. This plan fills all three gaps.
Output: README.md, three example manifests in examples/, and a test script in scripts/
</objective>

<execution_context>
@/Users/benvds/.claude/get-shit-done/workflows/execute-plan.md
@/Users/benvds/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@src/cli.rs (CLI flags and help text)
@src/init.rs (init template — the canonical manifest format)
@src/main.rs (full CLI flow)
@Cargo.toml (project metadata)
@install.sh (install script)
@.github/workflows/release.yml (release workflow)
@.skills.toml (real working manifest for reference)
</context>

<tasks>

<task type="auto">
  <name>Task 1: Create README.md and example manifests</name>
  <files>README.md, examples/minimal.skills.toml, examples/monorepo.skills.toml, examples/mixed.skills.toml</files>
  <action>
Create README.md with these sections (no badges, keep it practical):

1. **Header** — project name, one-line description: "Declarative skill synchronization for Claude Code. One command, your skills directory matches your manifest."

2. **Quick Start** — four steps: install (cargo install or install.sh curl), run `skills-sync --init`, edit .skills.toml, run `skills-sync`. Show expected output.

3. **Installation** — two methods:
   - From source: `cargo install --path .`
   - From release: `curl -fsSL https://raw.githubusercontent.com/OWNER/skills-sync/main/install.sh | sh`
   Note the OWNER placeholder that needs updating before first release.

4. **Manifest Format** — explain the three repo layouts with examples:
   - Single-skill repo (`"owner/repo"`) — has SKILL.md at root
   - Monorepo auto-detect (`"owner/multi-skill-repo"`) — subdirectories each with SKILL.md
   - Subdirectory pin (`"owner/repo/path/to/skill"`) — specific skill from a monorepo
   Link to the example files in examples/ for copy-paste starting points.

5. **CLI Reference** — table of all flags from cli.rs:
   `--init`, `--config <PATH>`, `--dry-run`, `--verbose`, `--global`, `--local`, `--force`, `--help`, `--version`
   Brief description for each.

6. **How It Works** — brief 3-4 sentence explanation: discovers manifest, uses GitHub Trees API to detect skills, compares SHA against lock file, installs/updates/removes via sparse git checkout. Lock file (.skills.lock) tracks installed state.

7. **Authentication** — explain: works unauthenticated (60 req/hr), set GITHUB_TOKEN or install gh CLI for 5000 req/hr. Show `export GITHUB_TOKEN=...`.

8. **Examples** — link to examples/ directory, briefly describe each example manifest.

9. **Development** — `cargo build`, `cargo test`, `cargo run -- --dry-run`.

Create three example manifests in examples/:

**examples/minimal.skills.toml** — Header comment explaining this is a minimal single-skill example. Contains one uncommented real repo entry (`anthropics/skills/skills/skill-creator`). Keep it very short.

**examples/monorepo.skills.toml** — Header comment explaining monorepo auto-detection. Contains one commented monorepo entry and one uncommented subdirectory-pinned entry to show both approaches.

**examples/mixed.skills.toml** — Header comment explaining mixed layouts. Contains 2-3 entries mixing single-skill, monorepo, and subdirectory pin formats. Use real repos from the existing .skills.toml as examples.
  </action>
  <verify>
    <automated>test -f README.md && test -f examples/minimal.skills.toml && test -f examples/monorepo.skills.toml && test -f examples/mixed.skills.toml && echo "All files exist"</automated>
  </verify>
  <done>README.md covers install, usage, manifest format, CLI reference, auth, and links to examples. Three example manifests exist covering all repo layout types.</done>
</task>

<task type="auto">
  <name>Task 2: Create manual integration test script</name>
  <files>scripts/test-sync.sh</files>
  <action>
Create scripts/test-sync.sh — a bash script that exercises the main CLI workflows in a temporary directory for safe, repeatable manual testing. Mark executable (chmod +x).

The script should:

1. Use `set -e` and create a temp directory via `mktemp -d`, with a trap to clean it up on exit.

2. Print a header explaining what it does.

3. **Test 1: Init** — Run `cargo run -- --init` in the temp dir. Verify .skills.toml was created. Print pass/fail.

4. **Test 2: Dry-run with example manifest** — Copy `examples/mixed.skills.toml` to the temp dir as `.skills.toml`. Run `cargo run -- --dry-run --config <path>`. Capture exit code (should be 2 for pending changes on first run). Print pass/fail.

5. **Test 3: Verbose dry-run** — Same manifest but with `--verbose --dry-run`. Just run it and print output. No exit code check needed (verbose is visual).

6. **Test 4: Actual sync** — Run `cargo run -- --force --config <path>` (force skips prompts). Check that .claude/skills/ directory was created with content. Print pass/fail.

7. **Test 5: Idempotent re-sync** — Run sync again, verify "everything up to date" in output. Print pass/fail.

8. Print summary: N/N tests passed.

Important implementation details:
- All `cargo run` invocations use `--` separator and work from the project root
- Use `--config` flag to point at the manifest in the temp dir
- The temp dir is the working directory for the tool (cd into it before running)
- Use the `examples/mixed.skills.toml` example which has real repo entries
- Set CARGO_TERM_COLOR=never in the script so color codes don't interfere with output parsing
- The script should work whether or not GITHUB_TOKEN is set (just slower without it)
- Use `cargo run --quiet --` to suppress build output
  </action>
  <verify>
    <automated>test -x scripts/test-sync.sh && head -1 scripts/test-sync.sh | grep -q "^#!/" && echo "Script exists and is executable"</automated>
  </verify>
  <done>scripts/test-sync.sh is executable, exercises init, dry-run, sync, and idempotent re-sync in a temp directory with automatic cleanup. A developer can run `./scripts/test-sync.sh` for a full manual smoke test.</done>
</task>

</tasks>

<verification>
- README.md exists and contains sections: Quick Start, Installation, Manifest Format, CLI Reference, Authentication
- Three example manifests exist in examples/ and are valid TOML (can test with `cargo run -- --dry-run --config examples/minimal.skills.toml`)
- scripts/test-sync.sh is executable and runs without errors (when GITHUB_TOKEN is available)
</verification>

<success_criteria>
- A new user reading README.md can install and use skills-sync without reading source code
- Example manifests are valid and demonstrate all three repo layout types
- Running `./scripts/test-sync.sh` exercises the core CLI workflows end-to-end in a disposable temp dir
</success_criteria>

<output>
After completion, create `.planning/quick/2-add-some-docs-tooling-and-examples-for-e/2-SUMMARY.md`
</output>
