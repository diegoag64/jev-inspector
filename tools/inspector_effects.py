"""Render the ASCII shader and verify that its startup animation settles."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug" / ("jev-inspector.exe" if os.name == "nt" else "jev-inspector"))
    parser.add_argument("--out", type=Path, default=ROOT / "artifacts/inspector-effects")
    args = parser.parse_args()
    binary, out = args.binary.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    results = []
    for theme in ("dark", "light"):
        directory = out / theme
        directory.mkdir(exist_ok=True)
        script = directory / "theme.json"
        script.write_text(json.dumps([{"t": 0, "type": "tap", "key": "theme-toggle"}] if theme == "light" else []), encoding="utf-8")
        command = [str(binary), "--fixture", "first-use", "--size", "1100x700",
                   "--scale", "1", "--fps", "4", "--frames", "13", "--out", str(directory),
                   "--script", str(script)]
        process = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=120)
        (directory / "render.log").write_text(process.stdout + process.stderr, encoding="utf-8")
        process.check_returncode()
        frames = sorted(directory.glob("*.png"))
        if len(frames) != 13:
            raise RuntimeError(f"Expected 13 frames, found {len(frames)} in {directory}")
        # Only decorative pixels in the fixed 1100x700 fixture. Exclude controls,
        # hover states and text from the motion comparison.
        region = (22, 8, 380, 46)
        def pixels(index):
            with Image.open(frames[index]) as frame:
                return frame.convert("RGB").crop(region)
        moving = ImageChops.difference(pixels(3), pixels(9)).getbbox() is not None
        settled = ImageChops.difference(pixels(9), pixels(12)).getbbox() is None
        if not moving or not settled:
            raise RuntimeError(f"{theme}: shader moved={moving}, settled={settled}")
        results.append({"theme": theme, "shader_pixels_changed": moving, "settled_pixels_identical": settled})
        print(f"PASS {theme}: ASCII pixels move, then settle", flush=True)
    (out / "report.json").write_text(json.dumps({
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "measurement": "Headless rendered pixels at virtual times. Not native GPU timing or OS reduced-motion certification.",
        "results": results,
    }, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
