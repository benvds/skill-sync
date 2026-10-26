"""Exercise release side effects only inside disposable Git repositories."""

import importlib.util
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("release", ROOT / "scripts/release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="skill-sync-release-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "checkout"
        self.root.mkdir()
        for name in release.VERSION_FIELDS:
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)
        scripts = self.root / "scripts"
        scripts.mkdir()
        shutil.copyfile(ROOT / "scripts/release.py", scripts / "release.py")
        # Stub expensive validation and image generation, keeping real Git/version edits.
        (scripts / "screenshots.py").write_text(
            "from pathlib import Path\n"
            "p = Path('docs/screenshots')\np.mkdir(parents=True, exist_ok=True)\n"
            "(p / 'sync.svg').write_text('<svg/>')\n")
        shutil.copyfile(ROOT / "install.sh", self.root / "install.sh")
        shutil.copyfile(ROOT / ".gitignore", self.root / ".gitignore")
        (self.root / "tests").mkdir()
        (self.root / "tests/test_validation.py").write_text(
            "import sys\nimport unittest\n"
            "sys.path.insert(0, 'scripts')\n"
            "from release import version_files\n\n"
            "class ValidationTests(unittest.TestCase):\n"
            "    def test_versions_agree(self):\n"
            "        version_files()\n")
        self.bin = Path(self.temp.name) / "bin"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text('#!/bin/sh\n[ "${FAIL_CARGO:-0}" != 1 ]\n')
        cargo.chmod(0o755)
        self.env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ["PATH"],
                        GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
                        GIT_AUTHOR_NAME="Release Test", GIT_AUTHOR_EMAIL="test@example.invalid",
                        GIT_COMMITTER_NAME="Release Test", GIT_COMMITTER_EMAIL="test@example.invalid")
        self.git("init", "-q", "-b", "main")
        self.git("add", ".")
        self.git("commit", "-qm", "Initial")
        self.current = release.version_files()[0]

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, env=self.env, text=True).strip()

    def script(self, *args):
        return subprocess.run([sys.executable, "scripts/release.py", *args], cwd=self.root,
                              env=self.env, capture_output=True, text=True)

    def assert_ok(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_bump_commit_and_annotated_tag_keep_dependencies_intact(self):
        old_lock = (self.root / "Cargo.lock").read_text()
        target = release.next_version(self.current, "minor")
        result = self.script("minor")
        self.assert_ok(result)
        # Older Python accepts empty suites; require actual validation everywhere.
        self.assertIn("Ran 1 test", result.stderr)
        self.assert_ok(self.script("--check", "--tag", "v" + target))
        self.assertEqual(self.git("status", "--porcelain"), "")
        self.assertEqual(self.git("cat-file", "-t", "v" + target), "tag")
        self.assertEqual(self.git("rev-parse", "HEAD"), self.git("rev-parse", "v" + target + "^{}"))
        self.assertEqual(self.git("log", "-1", "--format=%s"), "Release v" + target)
        pattern = release.VERSION_FIELDS["Cargo.lock"][0]
        new_lock = (self.root / "Cargo.lock").read_text()
        self.assertEqual(re.sub(pattern, "PACKAGE", old_lock), re.sub(pattern, "PACKAGE", new_lock))

    def test_dry_run_does_not_change_files_or_tags(self):
        head = self.git("rev-parse", "HEAD")
        self.assert_ok(self.script("patch", "--dry-run"))
        self.assertEqual(self.git("status", "--porcelain"), "")
        self.assertEqual(self.git("tag", "--list"), "")
        self.assertEqual(self.git("rev-parse", "HEAD"), head)

    def test_dirty_checkout_is_rejected_before_bumping(self):
        (self.root / "untracked.txt").write_text("Keep me")
        self.assertNotEqual(self.script("patch").returncode, 0)
        self.assert_ok(self.script("--check", "--tag", "v" + self.current))
        self.assertEqual(self.git("tag", "--list"), "")

    def test_existing_tag_is_rejected_before_bumping(self):
        target = release.next_version(self.current, "patch")
        self.git("tag", "v" + target)
        result = self.script("patch")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("already exists", result.stderr)
        self.assertEqual(self.git("status", "--porcelain"), "")

    def test_failed_validation_does_not_commit_or_tag(self):
        head = self.git("rev-parse", "HEAD")
        self.env["FAIL_CARGO"] = "1"
        self.assertNotEqual(self.script("patch").returncode, 0)
        self.assertEqual(self.git("rev-parse", "HEAD"), head)
        self.assertEqual(self.git("tag", "--list"), "")
        self.assertIn("Cargo.toml", self.git("status", "--porcelain"))

    def test_check_rejects_mismatched_docs_and_tags(self):
        self.assertNotEqual(self.script("--check", "--tag", "v99.99.99").returncode, 0)
        path = self.root / "README.md"
        path.write_text(path.read_text().replace(f"The package is currently `{self.current}`",
                                                "The package is currently `99.99.99`"))
        self.assertNotEqual(self.script("--check").returncode, 0)

    def test_push_publishes_branch_and_annotated_tag_to_local_remote(self):
        remote = Path(self.temp.name) / "origin.git"
        self.git("init", "-q", "--bare", str(remote))
        self.git("remote", "add", "origin", str(remote))
        target = release.next_version(self.current, "patch")
        self.assert_ok(self.script(target, "--push"))
        refs = self.git("ls-remote", "origin")
        self.assertIn("refs/heads/main", refs)
        self.assertIn("refs/tags/v" + target + "^{}", refs)

    def test_version_rules_and_argument_errors(self):
        self.assertEqual(release.next_version("1.2.3", "major"), "2.0.0")
        self.assertEqual(release.next_version("1.2.3", "minor"), "1.3.0")
        self.assertEqual(release.next_version("1.2.3", "patch"), "1.2.4")
        for target in ("1.2.3", "0.9.0", "01.2.4", "1.3.0-rc.1", "bad"):
            with self.subTest(target=target), self.assertRaises(ValueError):
                release.next_version("1.2.3", target)
        for args in (("--tag", "v1.0.0"), ("--check", "patch"), ("patch", "--dry-run", "--push")):
            self.assertNotEqual(self.script(*args).returncode, 0)


if __name__ == "__main__":
    unittest.main()
