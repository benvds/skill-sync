#!/usr/bin/env python3
"""Capture actual CLI examples as deterministic SVG terminal images in the README."""

import argparse
from html import escape
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap

ROOT = Path(__file__).resolve().parents[1]
START = "<!-- screenshots:start -->"
END = "<!-- screenshots:end -->"
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
GALLERY = """<!-- screenshots:start -->
### Preview and install

![Terminal showing a local preview and installation for Codex and Claude Code](docs/screenshots/sync.svg)

### Detect local edits

![Terminal showing offline status after editing an installed skill](docs/screenshots/status.svg)
<!-- screenshots:end -->"""


def terminal_svg(title, transcript):
    lines = []
    for line in transcript.splitlines():
        lines.extend(textwrap.wrap(line, width=100, expand_tabs=False,
                                   replace_whitespace=False, drop_whitespace=False) or [""])
    height = 76 + len(lines) * 23
    rows = []
    for index, line in enumerate(lines):
        color = "#a7f3d0" if line.startswith("$ ") else "#e2e8f0"
        rows.append(f'<tspan x="28" y="{79 + index * 23}" fill="{color}">{escape(line)}</tspan>')
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="1100" height="{height}" viewBox="0 0 1100 {height}" role="img" aria-labelledby="title">
<title id="title">{escape(title)}</title>
<rect width="1100" height="{height}" rx="12" fill="#101827"/>
<path d="M0 48H1100" stroke="#334155"/>
<g fill="#64748b"><circle cx="25" cy="24" r="5"/><circle cx="43" cy="24" r="5"/><circle cx="61" cy="24" r="5"/></g>
<text x="88" y="30" fill="#cbd5e1" font-family="monospace" font-size="16">{escape(title)}</text>
<text xml:space="preserve" font-family="'DejaVu Sans Mono', Menlo, Consolas, monospace" font-size="17">{''.join(rows)}</text>
</svg>
'''


def capture(binary):
    with tempfile.TemporaryDirectory(prefix="skill-sync-screenshots-") as directory:
        root = Path(directory).resolve()
        project = root / "project"
        source = project / "team-skills" / "review"
        source.mkdir(parents=True)
        (source / "SKILL.md").write_text("---\nname: review\ndescription: Review changes before shipping\n---\nCheck the diff and run focused tests.\n")
        (project / ".skills.toml").write_text('[sync]\nscope = "project"\nagents = ["codex", "claude-code"]\n\n[skills]\nrepos = ["local:team-skills"]\n')
        env = dict(os.environ, HOME=str(root / "home"), XDG_CONFIG_HOME=str(root / "config"),
                   XDG_CACHE_HOME=str(root / "cache"), NO_COLOR="1", CLICOLOR="0", TERM="dumb")
        for key in ("GITHUB_TOKEN", "GH_TOKEN", "CLICOLOR_FORCE", "FORCE_COLOR"):
            env.pop(key, None)

        def command(*args, expected=0):
            result = subprocess.run([str(binary), *args], cwd=project, env=env,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            if result.returncode != expected:
                raise ValueError(f"{args} exited {result.returncode}, expected {expected}:\n{result.stdout}")
            output = ANSI.sub("", result.stdout).replace(str(project), "~/demo").replace(str(root), "~")
            return "$ skill-sync " + " ".join(args) + "\n" + output.rstrip() + "\n"

        preview = command("--local", "--dry-run", expected=2)
        installed = command("--local")
        # Exercise the second run so screenshot capture also checks idempotence.
        installed += "\n" + command("--local")
        edited = project / ".agents" / "skills" / "review" / "SKILL.md"
        with edited.open("a") as handle:
            handle.write("\nA local review rule.\n")
        status = command("status", "--local", expected=2)
        return {
            "sync.svg": terminal_svg("skill-sync / preview and install", preview + "\n" + installed),
            "status.svg": terminal_svg("skill-sync / local edit detection", status),
        }


def update_readme(text):
    if text.count(START) != 1 or text.count(END) != 1:
        raise ValueError("README must contain exactly one screenshots:start/end marker pair")
    before, rest = text.split(START)
    _, after = rest.split(END)
    return before + GALLERY + after


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if committed screenshots or README embeds are stale")
    args = parser.parse_args()
    # Always build: captures must match the current source, not a stale binary.
    subprocess.run(["cargo", "build", "--locked", "--quiet"], cwd=ROOT, check=True)
    images = capture(ROOT / "target/debug/skill-sync")
    outputs = {ROOT / "docs/screenshots" / name: text for name, text in images.items()}
    readme = ROOT / "README.md"
    outputs[readme] = update_readme(readme.read_text())
    stale = [path for path, text in outputs.items() if not path.exists() or path.read_text() != text]
    if args.check:
        if stale:
            raise ValueError("Stale screenshots/embeds; run python3 scripts/screenshots.py: "
                             + ", ".join(str(path.relative_to(ROOT)) for path in stale))
        print("README screenshots are current.")
    else:
        for path, text in outputs.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        print("Updated README and " + ", ".join(images))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"screenshots: {error}", file=sys.stderr)
        sys.exit(1)
