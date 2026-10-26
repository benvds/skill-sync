import importlib.util
import os
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("screenshots", ROOT / "scripts/screenshots.py")
screenshots = importlib.util.module_from_spec(spec)
spec.loader.exec_module(screenshots)


class ScreenshotTests(unittest.TestCase):
    def test_svg_escapes_terminal_output_and_wraps_long_lines(self):
        svg = screenshots.terminal_svg("A & B", "$ skill-sync\n<file> & text\n" + "x" * 220)
        root = ET.fromstring(svg)
        self.assertIn("<file> & text", "".join(root.itertext()))
        self.assertTrue(all(len(row.text or "") <= 100 for row in root.iter("{http://www.w3.org/2000/svg}tspan")))

    def test_gallery_update_preserves_surrounding_readme(self):
        text = "Before\n" + screenshots.START + "\nOld gallery\n" + screenshots.END + "\nAfter"
        result = screenshots.update_readme(text)
        self.assertEqual(result, "Before\n" + screenshots.GALLERY + "\nAfter")
        self.assertEqual(screenshots.update_readme(result), result)
        for bad in ("No markers", text + screenshots.START):
            with self.assertRaises(ValueError):
                screenshots.update_readme(bad)

    @unittest.skipUnless(os.environ.get("SKILL_SYNC_TEST_BINARY"), "set SKILL_SYNC_TEST_BINARY for real CLI captures")
    def test_real_captures_are_repeatable_and_hide_temporary_paths(self):
        binary = Path(os.environ["SKILL_SYNC_TEST_BINARY"]).resolve()
        images = screenshots.capture(binary)
        self.assertEqual(images, screenshots.capture(binary))
        status = "".join(ET.fromstring(images["status.svg"]).itertext())
        self.assertIn("Modified:", status)
        self.assertIn("Unchanged:", status)
        self.assertIn("~/demo/", status)
        self.assertNotIn("skill-sync-screenshots-", "".join(images.values()))


if __name__ == "__main__":
    unittest.main()
