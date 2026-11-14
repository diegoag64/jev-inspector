import json
from pathlib import Path
import tempfile
import unittest


class FocusContrastTests(unittest.TestCase):
    def measure_rows(self, rows, background=0):
        try:
            from PIL import Image
            from tools.inspector_focus_contrast import measure
        except ImportError:
            self.skipTest("Install requirements.txt to exercise pixel comparisons")
        with tempfile.TemporaryDirectory() as folder:
            packet = Path(folder)
            tree = {"scale": 1, "elements": [{
                "key": "request-diagram:node:test:select",
                "focused": True, "focusVisible": True,
                "rect": [4, 10.25, 32, 12],
            }]}
            (packet / "focus.json").write_text(json.dumps(tree))
            image = Image.new("RGB", (40, 30), (background,) * 3)
            for y, value in enumerate(rows, start=8):
                for x in range(12, 28):
                    image.putpixel((x, y), (value,) * 3)
            image.save(packet / "focus.png")
            return measure(packet, "focus")

    def test_fractional_stroke_counts_multiple_contrasting_colors(self):
        for background, rows in [(0, [156, 245, 0]), (242, [106, 20, 242])]:
            with self.subTest(background=background):
                result = self.measure_rows(rows, background)
                self.assertTrue(result["ok"])
                self.assertAlmostEqual(result["stroke_fraction"], 2 / 3)

    def test_missing_faint_and_too_thin_strokes_fail(self):
        for rows in [[0, 0, 0], [60, 60, 60], [0, 245, 0]]:
            with self.subTest(rows=rows):
                self.assertFalse(self.measure_rows(rows)["ok"])


if __name__ == "__main__":
    unittest.main()
