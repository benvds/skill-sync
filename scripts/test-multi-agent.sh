#!/usr/bin/env bash
# Public GitHub smoke test; all configs, locks and installed copies live in a temporary tree.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cargo build --quiet --manifest-path "$PROJECT_ROOT/Cargo.toml"
python3 - "$PROJECT_ROOT/target/debug/skills-sync" <<'PY'
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile

binary = sys.argv[1]
with tempfile.TemporaryDirectory(prefix="skills-sync-multi-") as temporary:
    root = Path(temporary)
    home, config, project = root / "home", root / "config", root / "project"
    (config / "skills-sync").mkdir(parents=True)
    project.mkdir()
    user_config = config / "skills-sync/skills.toml"
    user_config.write_text('''[sync]
scope = "user"
agents = ["codex", "goose", "claude-code", "cursor"]
mode = "copy"
[skills]
repos = ["anthropics/skills/skills/skill-creator"]
''')
    (project / ".skills.toml").write_text('''[sync]
scope = "project"
agents = ["codex", "claude-code", "cursor"]
[skills]
repos = ["github/awesome-copilot/skills/github-issues"]
''')
    environment = dict(os.environ, HOME=str(home), XDG_CONFIG_HOME=str(config))
    def run(*args, expected=0):
        result = subprocess.run([binary, *args], cwd=project, env=environment,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if result.returncode != expected:
            raise AssertionError("{} exited {} (expected {})\n{}\n{}".format(args, result.returncode, expected, result.stdout, result.stderr))
        return result.stdout

    run("--dry-run", expected=2)
    assert not (home / ".agents").exists()
    assert not (project / ".agents").exists()
    assert not user_config.with_suffix(".lock").exists()
    print("[PASS] dry-run plans both scopes without installing")

    run("--force")
    personal = [home / folder / "skills/skill-creator/SKILL.md"
                for folder in (".agents", ".claude", ".cursor")]
    local = [project / folder / "skills/github-issues/SKILL.md"
             for folder in (".agents", ".claude", ".cursor")]
    assert all(path.is_file() and not path.parent.is_symlink() for path in personal + local)
    assert len({hashlib.sha256(path.read_bytes()).hexdigest() for path in personal}) == 1
    assert len({hashlib.sha256(path.read_bytes()).hexdigest() for path in local}) == 1
    assert not any((project / folder / "skills/skill-creator").exists()
                   for folder in (".agents", ".claude", ".cursor"))
    print("[PASS] three independent copies per scope; personal skills stay out of the project")

    user_lock = user_config.with_suffix(".lock")
    project_lock = project / ".skills.lock"
    assert user_lock.read_text().count("[[installations]]") == 3
    assert project_lock.read_text().count("[[installations]]") == 3
    assert "github-issues" not in user_lock.read_text()
    assert "skill-creator" not in project_lock.read_text()
    print("[PASS] locks track each skill/target pair and deduplicate Codex/Goose")

    before = (user_lock.read_bytes(), project_lock.read_bytes())
    output = run("--force")
    assert output.count("everything up to date") == 2
    assert (user_lock.read_bytes(), project_lock.read_bytes()) == before
    print("[PASS] repeated sync is idempotent, including locks")

    personal[1].unlink()
    # A missing resource in a still-present skill is a local modification and must be preserved.
    run("--global", "--force", expected=1)
    assert personal[0].is_file() and not personal[1].exists()
    import shutil
    shutil.rmtree(personal[1].parent)
    run("--global", "--force")
    assert personal[1].is_file()
    print("[PASS] partial edits are conflicts; an entirely missing copy is repaired independently")

    user_config.write_text(user_config.read_text().replace(
        'repos = ["anthropics/skills/skills/skill-creator"]', 'repos = []'))
    run("--global", "--force")
    assert all(not path.exists() for path in personal)
    assert all(path.is_file() for path in local)
    assert "[[installations]]" not in user_lock.read_text()
    assert project_lock.read_bytes() == before[1]
    print("[PASS] personal cleanup preserves every project copy and the project lock")
print("6/6 multi-agent smoke checks passed")
PY
