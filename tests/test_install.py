"""Offline installer checks using real archives and checksums, with mocked HTTP."""
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

INSTALLER = Path(__file__).resolve().parents[1] / "install.sh"


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.fixtures = self.root / "fixtures"
        self.fixtures.mkdir()
        self.downloads = self.root / "requests"
        self.install_dir = self.root / "path with spaces"
        self.install_dir.mkdir()
        (self.install_dir / "skill-sync").write_text("old binary")
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}",
                        FIXTURES=str(self.fixtures), REQUESTS=str(self.downloads),
                        TMPDIR=str(self.root), INSTALL_DIR=str(self.install_dir))
        self.env.pop("SKILL_SYNC_VERSION", None)
        self.executable("uname", '#!/bin/sh\ncase "$1" in -s) echo Darwin;; -m) echo arm64;; esac\n')
        self.executable("curl", '''#!/usr/bin/env python3
import os, pathlib, sys
args = sys.argv[1:]
url = next(a for a in args if a.startswith('https://'))
with open(os.environ['REQUESTS'], 'a') as log: log.write(url + '\\n')
fixture = pathlib.Path(os.environ['FIXTURES']) / url.rsplit('/', 1)[-1]
if not fixture.exists(): sys.exit(22)
if '-o' in args: pathlib.Path(args[args.index('-o') + 1]).write_bytes(fixture.read_bytes())
else: sys.stdout.buffer.write(fixture.read_bytes())
''')
        self.archive = "skill-sync-v1.2.3-aarch64-apple-darwin.tar.gz"
        with tarfile.open(self.fixtures / self.archive, "w:gz") as archive:
            data = b"#!/bin/sh\necho installed\n"
            member = tarfile.TarInfo("skill-sync")
            member.size = len(data)
            archive.addfile(member, io.BytesIO(data))
        digest = hashlib.sha256((self.fixtures / self.archive).read_bytes()).hexdigest()
        (self.fixtures / (self.archive + ".sha256")).write_text(f"{digest}  {self.archive}\n")
        (self.fixtures / "latest").write_text(json.dumps({"tag_name": "v1.2.3"}))

    def executable(self, name, content):
        path = self.bin / name
        path.write_text(content)
        path.chmod(0o755)

    def run_installer(self, *args):
        return subprocess.run(["sh", str(INSTALLER), *args], env=self.env,
                              capture_output=True, text=True)

    def assert_preserved(self, result):
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.install_dir / "skill-sync").read_text(), "old binary")
        self.assertFalse(list(self.root.glob("skill-sync.*")))
        self.assertFalse(list(self.install_dir.glob(".skill-sync.*")))

    def test_version_and_directory_options_install_verified_executable(self):
        destination = self.root / "custom directory"
        result = self.run_installer("--version", "v1.2.3", "--install-dir", str(destination))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(os.access(destination / "skill-sync", os.X_OK))
        self.assertEqual(subprocess.check_output([str(destination / "skill-sync")], text=True), "installed\n")
        self.assertNotIn("releases/latest", self.downloads.read_text())
        self.assertFalse(list(self.root.glob("skill-sync.*")))

    def test_latest_release_and_environment_options(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("releases/latest", self.downloads.read_text())

    def test_bad_checksum_preserves_existing_binary(self):
        (self.fixtures / (self.archive + ".sha256")).write_text(f"{'0' * 64}  {self.archive}\n")
        self.assert_preserved(self.run_installer("--version", "v1.2.3"))

    def test_missing_checksum_preserves_existing_binary(self):
        (self.fixtures / (self.archive + ".sha256")).unlink()
        self.assert_preserved(self.run_installer("--version", "v1.2.3"))

    def test_missing_archive_preserves_existing_binary(self):
        self.assert_preserved(self.run_installer("--version", "v9.9.9"))

    def test_directory_destination_is_not_treated_as_an_installed_binary(self):
        (self.install_dir / "skill-sync").unlink()
        (self.install_dir / "skill-sync").mkdir()
        result = self.run_installer("--version", "v1.2.3")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(list((self.install_dir / "skill-sync").iterdir()))
        self.assertFalse(list(self.root.glob("skill-sync.*")))

    def test_invalid_option_and_platform_fail_before_network(self):
        self.assertNotEqual(self.run_installer("--version").returncode, 0)
        self.assertNotEqual(self.run_installer("--unknown").returncode, 0)
        self.executable("uname", '#!/bin/sh\necho unsupported\n')
        self.assertNotEqual(self.run_installer().returncode, 0)
        self.assertFalse(self.downloads.exists())


if __name__ == "__main__":
    unittest.main()
