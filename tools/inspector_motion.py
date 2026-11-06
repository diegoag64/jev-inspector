"""Replay mixed inspector layouts; save a Rune motion trace, frame strip and CPU report."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def summarize(path: Path) -> dict:
    metadata, surfaces, cpu = {}, set(), []
    fading = 0
    final = {}
    with path.open(encoding="utf-8") as stream:
        header = json.loads(next(stream))
        if header.get("clock") != "virtual":
            raise ValueError("Expected a deterministic headless trace")
        for line in stream:
            frame = json.loads(line)
            for item in frame.get("m", []):
                metadata[item["id"]] = item["nm"]
            for element in frame.get("e", []):
                if not all(math.isfinite(v) for v in element[1:6]):
                    raise ValueError("Non-finite element geometry or opacity")
                key = metadata.get(element[0], "").split("/")[-1]
                if key.startswith("detail-content-"):
                    surfaces.add(key)
                    fading += 0 < element[5] < 1
            final = frame["quality"]
            cpu.append(final["cpu_ms"])
    # Trace element rows are deltas, not a full scene. Runtime tests separately
    # verify that old surfaces disappear and only one remains during interruption.
    expected = {"detail-content-4-0-ready", "detail-content-2-0-ready",
                "detail-content-1-3-ready", "detail-content-7-1-ready"}
    if not expected.issubset(surfaces) or fading == 0 or final.get("animating"):
        raise ValueError("Replay missed a required layout/fade or did not settle")
    ordered = sorted(cpu)
    return {"frames": len(cpu), "observed_surfaces": sorted(surfaces),
            "fading_surface_samples": fading, "settled_at_end": True,
            "cpu_ms": {"median": statistics.median(cpu),
                       "p95": ordered[math.ceil(len(cpu) * .95) - 1], "max": max(cpu)}}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug" / ("jev-inspector.exe" if os.name == "nt" else "jev-inspector"))
    parser.add_argument("--out", type=Path, default=ROOT / "artifacts/inspector-motion")
    args = parser.parse_args()
    binary, out = args.binary.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    base = [str(binary), "--fixture", "example-return-review", "--size", "1100x700", "--scale", "1", "--fps", "60"]
    script = ROOT / "tests/inspector-mixed-motion.script.json"
    trace = out / "trace.jsonl"

    def run(extra, log):
        result = subprocess.run(base + extra, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=120)
        (out / log).write_text(result.stdout + result.stderr, encoding="utf-8")
        result.check_returncode()

    run(["--script", str(script), "--frames", "540", "--trace", str(trace)], "trace.log")
    report = summarize(trace)
    short = out / "first-transition.script.json"
    short.write_text(json.dumps([e for e in json.loads(script.read_text()) if e["t"] <= .5]), encoding="utf-8")
    run(["--script", str(short), "--start", "0.45", "--frames", "36", "--every", "3",
         "--strip", str(out / "transition.png"), "--crop", "580,110,510,540", "--cols", "3"], "strip.log")
    report.update(platform=platform.platform(), binary=str(binary),
                  binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                  measurement="Forced headless CPU frames at a virtual 60 Hz. Not native display FPS, GPU completion time or input latency.")
    report["build_directory"] = binary.parent.name
    try:
        report["source_revision"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, stderr=subprocess.DEVNULL).strip()
        report["working_tree_dirty"] = bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True))
    except (OSError, subprocess.CalledProcessError):
        report["source_revision"] = None
        report["working_tree_dirty"] = None
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
