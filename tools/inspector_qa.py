"""Render the native inspector's fixtures and record bounded headless timing data."""
from __future__ import annotations

import argparse
import datetime
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
CASES = {
    "overview-dark": [],
    "response-small-light": ["--size", "1100x700", "--script", "tests/inspector-json-response.script.json"],
    "json-find": ["--script", "tests/inspector-workspace-find.script.json"],
    "json-tree": ["--script", "tests/inspector-workspace-tree.script.json"],
    "compare": ["--script", "tests/inspector-workspace-compare.script.json"],
    "export": ["--script", "tests/inspector-workspace-export.script.json"],
    "diagram-expanded": ["--diagram", "--script", "tests/inspector-diagram-expanded.script.json"],
    "upstream-error": ["--script", "tests/inspector-workspace-error.script.json"],
    "help": ["--script", "tests/inspector-workspace-help.script.json"],
    "empty": ["--script", "tests/inspector-workspace-empty.script.json"],
    "saved-session": ["--fixture", "saved"],
    "summary-only": ["--fixture", "summary-only"],
    "disconnected": ["--fixture", "disconnected"],
    "paused": ["--fixture", "paused"],
    "missing-compare": ["--fixture", "missing-compare"],
    "invalid-query": ["--fixture", "invalid-query"],
    "long-json-small": ["--fixture", "long-json", "--size", "1100x700", "--script", "tests/inspector-workspace-find.script.json"],
    "export-progress": ["--fixture", "export-progress"],
    "export-cancelled": ["--fixture", "export-cancelled"],
    "export-failed": ["--fixture", "export-failed"],
    "long-labels-small": ["--fixture", "long-labels", "--size", "1100x700"],
    "tree-small": ["--size", "1100x700", "--script", "tests/inspector-workspace-tree.script.json"],
    "compare-small-split": ["--size", "1100x700", "--script", "tests/inspector-workspace-compare-split.script.json"],
    "compare-small-expanded": ["--size", "1100x700", "--script", "tests/inspector-workspace-compare-expanded.script.json"],
    "help-small": ["--size", "1100x700", "--script", "tests/inspector-workspace-help.script.json"],
}


def verify_state(name: str, tree_path: Path) -> list[str]:
    """A rendered PNG is not enough: verify the scripted workflow reached its destination."""
    tree = json.loads(tree_path.read_text(encoding="utf-8"))["accessibility"]
    def walk(node):
        yield node
        for child in node.get("children", []):
            yield from walk(child)
    nodes = list(walk(tree))
    keyed = {n["key"]: n for n in nodes if "key" in n}
    names = "\n".join(n.get("name", "") for n in nodes)
    errors = []
    selected_tabs = {"overview-dark": 0, "response-small-light": 2, "json-find": 2,
                     "json-tree": 2, "compare": 4, "diagram-expanded": 3,
                     "missing-compare": 4, "long-json-small": 2, "tree-small": 2, "compare-small-split": 4, "compare-small-expanded": 4}
    if name in selected_tabs:
        key = f"tab-{selected_tabs[name]}"
        if "selected" not in keyed.get(key, {}).get("states", []):
            errors.append(f"Expected selected {key}")
    required_keys = {"json-find": "response-json:search", "long-json-small": "response-json:search",
                     "json-tree": "copy-json-value", "compare": "exchange-diff",
                     "export": "export-dialog", "help": "help-dialog", "empty": "empty-restore",
                     "tree-small": "copy-json-value", "compare-small-split": "exchange-diff", "help-small": "help-dialog"}
    if name in required_keys and required_keys[name] not in keyed:
        errors.append(f"Missing {required_keys[name]}")
    if name in ("compare-small-split", "compare-small-expanded") and keyed.get("diff-mode", {}).get("name") != "Unified":
        errors.append("Expected split comparison mode")
    if name == "compare-small-expanded" and "requests" in keyed:
        errors.append("Expected expanded comparison")
    required_text = {"saved-session": "SAVED WITH BODIES", "summary-only": "summary only",
                     "disconnected": "CAPTURE UNKNOWN", "paused": "List paused",
                     "missing-compare": "No captured bodies available to compare",
                     "invalid-query": "Use status:200", "long-json-small": "Preview limited",
                     "export-progress": "Working", "export-cancelled": "File operation cancelled",
                     "upstream-error": "429", "export-failed": "Could not save session", "long-labels-small": "SAVED SESSION"}
    if name in required_text and required_text[name] not in names:
        errors.append(f"Missing state text: {required_text[name]}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target" / "debug" / ("jev-inspector.exe" if os.name == "nt" else "jev-inspector"))
    parser.add_argument("--out", type=Path, default=ROOT / "artifacts" / "inspector-qa")
    parser.add_argument("--case", choices=list(CASES), action="append", dest="cases")
    parser.add_argument("--scale", type=float, default=1)
    args = parser.parse_args()
    binary, out = args.binary.resolve(), args.out.resolve()
    if not binary.is_file():
        parser.error(f"Build the inspector first: cargo build --workspace --locked. Missing {binary}")
    if not 0.5 <= args.scale <= 3:
        parser.error("--scale must be between 0.5 and 3")
    out.mkdir(parents=True, exist_ok=True)
    results = []
    for name in args.cases or CASES:
        options = list(CASES[name])
        events = []
        if "--script" in options:
            i = options.index("--script")
            events = json.loads((ROOT / options[i + 1]).read_text(encoding="utf-8"))
            del options[i:i + 2]
        # Let initial viewport layout and control transitions settle before scripted input.
        events = [{**event, "t": event["t"] + 0.35} for event in events]
        events.append({"t": 1.8, "type": "move", "x": 10, "y": 90})
        script = out / f"{name}.script.json"
        script.write_text(json.dumps(events), encoding="utf-8")
        command = [str(binary), "--demo", "--scale", str(args.scale), "--at", "2.1", *options, "--script", str(script),
                   "--snap", str(out / f"{name}.png"), "--tree", str(out / f"{name}.json")]
        started = time.monotonic()
        try:
            process = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=120)
            log = process.stdout + process.stderr
            ok = process.returncode == 0 and (out / f"{name}.png").is_file()
            if ok:
                errors = verify_state(name, out / f"{name}.json")
                ok = not errors
                log += "\n" + "\n".join(errors)
        except subprocess.TimeoutExpired:
            log, ok = "Rendering exceeded 120 seconds.", False
        (out / f"{name}.log").write_text(log, encoding="utf-8")
        match = re.search(r"frame cpu mean ([\d.]+) ms p50 ([\d.]+) p95 ([\d.]+) max ([\d.]+)", log)
        result = {"case": name, "ok": ok, "wall_seconds": round(time.monotonic() - started, 3),
                  "screenshot": f"{name}.png", "tree": f"{name}.json", "log": f"{name}.log"}
        if match:
            result["forced_headless_cpu_ms"] = dict(zip(("mean", "p50", "p95", "max"), map(float, match.groups())))
        results.append(result)
        print(f"{'PASS' if ok else 'FAIL'} {name} ({result['wall_seconds']:.1f}s)", flush=True)
    report = {"created_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "platform": platform.platform(),
              "machine": platform.machine(), "binary": str(binary), "scale": args.scale,
              "measurement": "Forced headless frames, including scripted transitions. Not interactive FPS or network latency. Compare the same build profile and scale.",
              "results": results}
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return 0 if all(r["ok"] for r in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
