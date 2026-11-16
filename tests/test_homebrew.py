"""Validate the generated Homebrew formula without contacting GitHub."""

import hashlib
import importlib.util
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("homebrew", ROOT / "scripts/homebrew.py")
homebrew = importlib.util.module_from_spec(spec)
spec.loader.exec_module(homebrew)


class HomebrewTests(unittest.TestCase):
    def test_checked_in_formula_covers_published_targets_and_runtime_dependency(self):
        tag, checksums = homebrew.check_formula(ROOT / "Formula/skill-sync.rb")
        self.assertRegex(tag, r"^v[0-9]+\.[0-9]+\.[0-9]+$")
        expected = {homebrew.asset_name(tag, target) for _, _, target in homebrew.TARGETS}
        self.assertEqual(set(checksums), expected)
        for digest in checksums.values():
            self.assertRegex(digest, r"^[0-9a-f]{64}$")
        text = (ROOT / "Formula/skill-sync.rb").read_text()
        self.assertIn(f'  version "{tag.removeprefix("v")}"', text)
        self.assertEqual(re.findall(r'^\s*depends_on "([^"]+)"', text, re.MULTILINE), ["git"])
        self.assertIn('bin.install "skill-sync"', text)
        self.assertIn('system "#{bin}/skill-sync", "--version"', text)
        for _, _, target in homebrew.TARGETS:
            self.assertIn(f"/download/{tag}/{homebrew.asset_name(tag, target)}", text)

    def test_update_reads_sidecars_and_generates_a_checkable_formula(self):
        tag = "v9.8.7"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            expected = {}
            for _, _, target in homebrew.TARGETS:
                asset = homebrew.asset_name(tag, target)
                digest = hashlib.sha256(asset.encode()).hexdigest()
                expected[asset] = digest
                (root / f"{asset}.sha256").write_text(f"{digest}  {asset}\n")
            formula = root / "skill-sync.rb"
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts/homebrew.py"), "--update", "--tag", tag,
                 "--checksum-dir", str(root), "--formula", str(formula)],
                cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            checked_tag, checksums = homebrew.check_formula(formula)
            self.assertEqual(checked_tag, tag)
            self.assertEqual(checksums, expected)
            text = formula.read_text()
            self.assertIn('on_macos do\n    on_arm do', text)
            self.assertIn('on_linux do\n    on_intel do', text)

    def test_check_rejects_hand_edits_and_stale_tags(self):
        text = (ROOT / "Formula/skill-sync.rb").read_text()
        with tempfile.TemporaryDirectory() as directory:
            formula = Path(directory) / "skill-sync.rb"
            missing_dependency = text.replace('  depends_on "git"\n\n', "")
            formula.write_text(missing_dependency)
            with self.assertRaisesRegex(ValueError, "generated form"):
                homebrew.check_formula(formula)
            formula.write_text(text)
            with self.assertRaisesRegex(ValueError, "expected v99.99.99"):
                homebrew.check_formula(formula, "v99.99.99")
            _, checksums = homebrew.check_formula(formula)
            duplicate = list(checksums.values())
            formula.write_text(text.replace(duplicate[1], duplicate[0]))
            with self.assertRaisesRegex(ValueError, "reuses a SHA-256"):
                homebrew.check_formula(formula)

    def test_invalid_release_metadata_is_rejected(self):
        tag = "v1.2.3"
        asset = homebrew.asset_name(tag, homebrew.TARGETS[0][2])
        digest = "a" * 64
        with self.assertRaisesRegex(ValueError, "names"):
            homebrew.parse_sidecar(f"{digest}  wrong.tar.gz\n", asset)
        with self.assertRaisesRegex(ValueError, "Invalid SHA-256"):
            homebrew.parse_sidecar(f"not-a-digest  {asset}\n", asset)
        with self.assertRaisesRegex(ValueError, "one .* line"):
            homebrew.parse_sidecar(f"{digest}  {asset}\n{digest}  {asset}\n", asset)
        for invalid in ("1.2", "01.2.3", "v1.2.3-rc.1", "vbad"):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                homebrew.normalize_tag(invalid)

    def test_sidecar_fetch_reports_missing_checksums(self):
        with tempfile.TemporaryDirectory() as directory, self.assertRaises(FileNotFoundError):
            homebrew.fetch_checksums("v1.2.3", Path(directory))


if __name__ == "__main__":
    unittest.main()
