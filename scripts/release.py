#!/usr/bin/env python3
"""Bump package/docs versions, validate, commit and create an annotated Git tag."""

import argparse
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SEMVER = r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
# Match only this package's versions; dependency and schema versions stay intact.
VERSION_FIELDS = {
    "Cargo.toml": [r'(\[package\][\s\S]*?\nversion = ")([^"\n]+)(")'],
    "Cargo.lock": [r'(\[\[package\]\]\nname = "skill-sync"\nversion = ")([^"\n]+)(")'],
    "README.md": [
        r'(sh install-skill-sync\.sh --version v)([0-9.]+)( )',
        r'(The package is currently `)([0-9.]+)(`)',
    ],
    "docs/PRD.md": [r'(the current Cargo package is `)([0-9.]+)(`)'],
    "docs/ROADMAP.md": [r'(The current package version is `)([0-9.]+)(`)'],
}


def run(*args, capture=False, env=None):
    result = subprocess.run(args, cwd=ROOT, check=True, text=True, env=env,
                            stdout=subprocess.PIPE if capture else None)
    return result.stdout.strip() if capture else None


def version_tuple(version):
    if not re.fullmatch(SEMVER, version):
        raise ValueError(f"Expected a stable version X.Y.Z, got {version!r}")
    return tuple(map(int, version.split(".")))


def version_files():
    contents = {name: (ROOT / name).read_text() for name in VERSION_FIELDS}
    versions = []
    for name, patterns in VERSION_FIELDS.items():
        for pattern in patterns:
            matches = list(re.finditer(pattern, contents[name]))
            if len(matches) != 1:
                raise ValueError(f"Expected one version field in {name}: {pattern}")
            versions.append((name, matches[0].group(2)))
    current = versions[0][1]
    version_tuple(current)
    for name, version in versions:
        if version != current:
            raise ValueError(f"Version mismatch: {name} has {version}, expected {current}")
    return current, contents


def next_version(current, requested):
    parts = list(version_tuple(current))
    if requested in ("major", "minor", "patch"):
        index = ("major", "minor", "patch").index(requested)
        parts[index] += 1
        parts[index + 1:] = [0] * (2 - index)
        return ".".join(map(str, parts))
    target = requested.removeprefix("v")
    if version_tuple(target) <= tuple(parts):
        raise ValueError(f"New version must be greater than {current}")
    return target


def validate():
    run("cargo", "fmt", "--all", "--check")
    run("cargo", "test", "--locked")
    run("cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings")
    run("sh", "-n", "install.sh")
    env = dict(os.environ, SKILL_SYNC_TEST_BINARY=str(ROOT / "target/debug/skill-sync"))
    run(sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_*.py", "-v", env=env)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", nargs="?", help="patch, minor, major, or an explicit X.Y.Z")
    parser.add_argument("--dry-run", action="store_true", help="show the bump without writing or running checks")
    parser.add_argument("--check", action="store_true", help="check version consistency without changing files")
    parser.add_argument("--tag", help="with --check, require this exact vX.Y.Z tag")
    parser.add_argument("--push", action="store_true", help="atomically push the current branch and tag to origin")
    args = parser.parse_args()
    if args.check:
        if args.version or args.dry_run or args.push:
            parser.error("--check cannot be combined with a bump, --dry-run or --push")
    elif not args.version or args.tag:
        parser.error("specify a version bump, or use --check [--tag vX.Y.Z]")
    if args.push and args.dry_run:
        parser.error("--push cannot be combined with --dry-run")

    current, contents = version_files()
    if args.check:
        if args.tag and args.tag != f"v{current}":
            raise ValueError(f"Tag {args.tag!r} must match package version v{current}")
        print(f"Package, lockfile and documentation agree: v{current}")
        return

    target = next_version(current, args.version)
    tag = f"v{target}"
    tags = run("git", "tag", "--list", tag, capture=True)
    if tags:
        raise ValueError(f"Tag {tag} already exists")
    print(f"Release v{current} -> {tag}", flush=True)
    print("Update: " + ", ".join(contents) + "; regenerate README screenshots", flush=True)
    if args.dry_run:
        print("Would validate, commit and create an annotated tag. Nothing changed.")
        return

    if run("git", "status", "--porcelain", "--untracked-files=all", capture=True):
        raise ValueError("Commit or stash existing changes before releasing (including untracked files)")
    run("git", "symbolic-ref", "--quiet", "HEAD", capture=True)
    # Keep failed validation reviewable: leave bumped files, without a commit/tag.
    for name, text in contents.items():
        for pattern in VERSION_FIELDS[name]:
            text = re.sub(pattern, lambda m: m.group(1) + target + m.group(3), text)
        (ROOT / name).write_text(text)
    run(sys.executable, "scripts/screenshots.py")
    validate()
    version_files()
    run("git", "add", "--", *contents, "docs/screenshots")
    run("git", "commit", "-m", f"Release {tag}")
    run("git", "tag", "-a", tag, "-m", f"Release {tag}")
    print(f"Created {tag}.", flush=True)
    if args.push:
        run("git", "push", "--atomic", "origin", "HEAD", f"refs/tags/{tag}")
    else:
        print(f"Publish with: git push --atomic origin HEAD refs/tags/{tag}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"release: {error}", file=sys.stderr)
        sys.exit(1)
