#!/usr/bin/env python3
"""Render and validate the Homebrew formula for published skill-sync releases."""

import argparse
from pathlib import Path
import re
import sys
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = "benvds/skill-sync"
PACKAGE = "skill-sync"
FORMULA = ROOT / "Formula/skill-sync.rb"
SEMVER = r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
# Only these archives are built by the release workflow.
TARGETS = (
    ("macos", "arm", "aarch64-apple-darwin"),
    ("linux", "intel", "x86_64-unknown-linux-musl"),
)
CHECKSUM = re.compile(r"^([0-9a-f]{64})$")
ENTRY = re.compile(r'url "(?P<url>[^"\n]+)"\s*\n\s*sha256 "(?P<sha>[^"\n]+)"')
TAG = re.compile(r"/releases/download/(v[^/]+)/")


def package_version():
    text = (ROOT / "Cargo.toml").read_text()
    match = re.search(r'\[package\][\s\S]*?\nversion = "([^"\n]+)"', text)
    if not match or not re.fullmatch(SEMVER, match.group(1)):
        raise ValueError("Cargo.toml does not contain a stable package version")
    return match.group(1)


def normalize_tag(tag):
    version = tag.removeprefix("v")
    if not re.fullmatch(SEMVER, version):
        raise ValueError(f"Expected a stable release tag such as v1.2.3, got {tag!r}")
    return f"v{version}"


def asset_name(tag, target):
    return f"{PACKAGE}-{tag}-{target}.tar.gz"


def asset_url(tag, asset):
    return f"https://github.com/{REPOSITORY}/releases/download/{tag}/{asset}"


def parse_sidecar(content, asset):
    lines = [line.split() for line in content.splitlines() if line.strip()]
    if len(lines) != 1 or len(lines[0]) != 2:
        raise ValueError(f"Expected one '<sha256>  {asset}' line in {asset}.sha256")
    digest, name = lines[0]
    if name.lstrip("*") != asset:
        raise ValueError(f"Checksum names {name!r}, expected {asset!r}")
    if not CHECKSUM.fullmatch(digest):
        raise ValueError(f"Invalid SHA-256 checksum for {asset}: {digest!r}")
    return digest


def fetch_checksums(tag, checksum_dir=None):
    checksums = {}
    for _, _, target in TARGETS:
        asset = asset_name(tag, target)
        if checksum_dir:
            path = Path(checksum_dir) / f"{asset}.sha256"
            content = path.read_text()
        else:
            request = urllib.request.Request(
                asset_url(tag, asset) + ".sha256",
                headers={"User-Agent": f"{PACKAGE}-homebrew-maintenance"},
            )
            with urllib.request.urlopen(request, timeout=30) as response:
                content = response.read().decode()
        checksums[asset] = parse_sidecar(content, asset)
    return checksums


def render(tag, checksums):
    tag = normalize_tag(tag)
    version = tag.removeprefix("v")
    lines = [
        "class SkillSync < Formula",
        '  desc "Sync Agent Skills into the directories used by AI coding agents"',
        f'  homepage "https://github.com/{REPOSITORY}"',
        f'  version "{version}"',
        "",
    ]
    for os_name, arch, target in TARGETS:
        asset = asset_name(tag, target)
        if asset not in checksums:
            raise ValueError(f"Missing SHA-256 checksum for {asset}")
        lines.extend((
            f"  on_{os_name} do",
            f"    on_{arch} do",
            f'      url "{asset_url(tag, asset)}"',
            f'      sha256 "{checksums[asset]}"',
            "    end",
            "  end",
            "",
        ))
    lines.extend((
        '  depends_on "git"',
        "",
        "  def install",
        f'    bin.install "{PACKAGE}"',
        "  end",
        "",
        "  test do",
        f'    system "#{{bin}}/{PACKAGE}", "--version"',
        "  end",
        "end",
        "",
    ))
    return "\n".join(lines)


def check_formula(path=FORMULA, expected_tag=None):
    text = Path(path).read_text()
    entries = list(ENTRY.finditer(text))
    if len(entries) != len(TARGETS):
        raise ValueError(f"Expected {len(TARGETS)} url/sha256 pairs in {path}, found {len(entries)}")
    tags = {match.group(1) for match in TAG.finditer(text)}
    if len(tags) != 1:
        raise ValueError(f"Expected one release tag in {path}, found {sorted(tags)}")
    tag = tags.pop()
    normalize_tag(tag)
    if expected_tag and tag != normalize_tag(expected_tag):
        raise ValueError(f"Formula contains {tag}, expected {normalize_tag(expected_tag)}")
    checksums = {}
    for entry in entries:
        asset = entry.group("url").rsplit("/", 1)[-1]
        if asset in checksums:
            raise ValueError(f"Duplicate release asset in formula: {asset}")
        if not CHECKSUM.fullmatch(entry.group("sha")):
            raise ValueError(f"Invalid SHA-256 checksum for {asset}: {entry.group('sha')!r}")
        checksums[asset] = entry.group("sha")
    if len(set(checksums.values())) != len(checksums):
        raise ValueError(f"Formula reuses a SHA-256 checksum across different archives in {path}")
    expected = render(tag, checksums)
    if text != expected:
        raise ValueError(
            f"{path} is not in the generated form; run scripts/homebrew.py --update")
    return tag, checksums


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true", help="validate the checked-in formula without network access")
    action.add_argument("--update", action="store_true", help="fetch release checksums and rewrite the formula")
    parser.add_argument("--tag", help="release tag such as v1.2.3 (default: the Cargo.toml version)")
    parser.add_argument("--formula", type=Path, default=FORMULA, help=f"formula path (default: {FORMULA.relative_to(ROOT)})")
    parser.add_argument("--checksum-dir", type=Path, help="read <asset>.sha256 files from this directory instead of GitHub")
    parser.add_argument("--dry-run", action="store_true", help="print the updated formula without writing it")
    args = parser.parse_args()
    if args.check and (args.checksum_dir or args.dry_run):
        parser.error("--check cannot be combined with --checksum-dir or --dry-run")
    if args.checksum_dir and not args.update:
        parser.error("--checksum-dir requires --update")

    if args.check:
        checked_tag, checksums = check_formula(args.formula, args.tag)
        print(f"Homebrew formula is valid for {checked_tag} with {len(checksums)} platform archives")
        return

    tag = normalize_tag(args.tag or package_version())
    text = render(tag, fetch_checksums(tag, args.checksum_dir))
    if args.dry_run:
        print(text, end="")
        return
    args.formula.parent.mkdir(parents=True, exist_ok=True)
    args.formula.write_text(text)
    print(f"Updated {args.formula} for {tag}")
    print("Copy it to the Formula/ directory of benvds/homebrew-tap and commit there.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, urllib.error.URLError) as error:
        print(f"homebrew: {error}", file=sys.stderr)
        sys.exit(1)
