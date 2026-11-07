"""Check rendered diagram focus strokes in both themes and at Fit/working zoom."""
from __future__ import annotations

import argparse
from collections import Counter
import json
import math
from pathlib import Path

from PIL import Image

CASES = (
    "example-diagram-keyboard", "focus-heading-light",
    "focus-heading-fit", "focus-heading-fit-light",
    "example-properties-top", "focus-properties-light",
    "inactive-properties-track", "focus-properties-fit-light",
)


def luminance(rgb):
    values = [v / 255 for v in rgb]
    values = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4 for v in values]
    return sum(v * weight for v, weight in zip(values, (.2126, .7152, .0722)))


def measure(packet: Path, case: str) -> dict:
    tree = json.loads((packet / (case + ".json")).read_text(encoding="utf-8"))
    focused = [e for e in tree["elements"] if e.get("focused")]
    if len(focused) != 1 or not focused[0].get("focusVisible"):
        raise ValueError(f"{packet}/{case}: expected one visible keyboard focus indicator")
    element = focused[0]
    if not element.get("key", "").startswith("request-diagram:node:"):
        raise ValueError(f"{packet}/{case}: focus is not inside a diagram card")
    x, y, width, _ = element["rect"]
    scale = tree["scale"]
    # Sample the outside top stroke, away from corners and text. Sample its
    # neighboring card background separately; do not assume a palette color.
    strip = (math.ceil((x + width * .25) * scale), math.floor((y - 2) * scale),
             math.floor((x + width * .75) * scale), math.ceil(y * scale))
    background_point = (int((x + width / 2) * scale), int((y - 4) * scale))
    with Image.open(packet / (case + ".png")) as source:
        image = source.convert("RGB")
        if not (0 <= strip[0] < strip[2] <= image.width and
                0 <= background_point[1] < strip[1] < strip[3] <= image.height):
            raise ValueError(f"{packet}/{case}: focus sample is clipped")
        colors = Counter(image.crop(strip).getdata())
        stroke, count = colors.most_common(1)[0]
        background = image.getpixel(background_point)
    low, high = sorted((luminance(stroke), luminance(background)))
    ratio = (high + .05) / (low + .05)
    coverage = count / sum(colors.values())
    return {"packet": str(packet), "case": case, "scale": scale,
            "key": element["key"], "strip_px": strip, "background_px": background_point,
            "stroke_rgb": stroke, "background_rgb": background, "contrast": ratio,
            "stroke_fraction": coverage, "ok": ratio >= 3 and coverage >= .5}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", type=Path, action="append", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    results = [measure(packet, case) for packet in args.packet for case in CASES]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({"results": results,
        "scope": "Rendered top focus stroke versus adjacent card background. Not a full accessibility audit."},
        indent=2) + "\n", encoding="utf-8")
    for result in results:
        print(f"{'PASS' if result['ok'] else 'FAIL'} {result['case']} "
              f"scale={result['scale']} contrast={result['contrast']:.2f}:1")
    return 0 if all(result["ok"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
