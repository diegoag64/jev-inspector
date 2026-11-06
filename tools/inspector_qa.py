"""Render the native inspector's fixtures and record bounded headless timing data."""
from __future__ import annotations

import argparse
import datetime
import hashlib
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
    "long-scalar-small": ["--fixture", "long-json", "--size", "1100x700"],
    "export-progress": ["--fixture", "export-progress"],
    "export-cancelled": ["--fixture", "export-cancelled"],
    "export-failed": ["--fixture", "export-failed"],
    "long-labels-small": ["--fixture", "long-labels", "--size", "1100x700"],
    "tree-small": ["--size", "1100x700", "--script", "tests/inspector-workspace-tree.script.json"],
    "compare-small-split": ["--size", "1100x700", "--script", "tests/inspector-workspace-compare-split.script.json"],
    "compare-small-expanded": ["--size", "1100x700", "--script", "tests/inspector-workspace-compare-expanded.script.json"],
    "help-small": ["--size", "1100x700", "--script", "tests/inspector-workspace-help.script.json"],
    "loading-overview": ["--fixture", "loading"],
    "loading-overview-small": ["--fixture", "loading", "--size", "1100x700"],
    "loading-request": ["--fixture", "loading-request", "--size", "1100x700"],
    "loading-response-light": ["--fixture", "loading", "--size", "1100x700", "--script", "tests/inspector-json-response.script.json"],
    "loading-diagram": ["--fixture", "loading-diagram", "--size", "1100x700"],
    "loading-compare": ["--fixture", "loading-compare", "--size", "1100x700"],
    "detail-failed": ["--fixture", "detail-failed", "--size", "1100x700"],
    "disconnected-empty": ["--fixture", "disconnected-empty", "--size", "1100x700"],
    "pending-response": ["--fixture", "pending-response", "--size", "1100x700"],
    "empty-response": ["--fixture", "empty-response", "--size", "1100x700"],
    "first-use": ["--fixture", "first-use", "--size", "1100x700"],
    "saved-empty": ["--fixture", "saved-empty", "--size", "1100x700"],
    "no-matches": ["--fixture", "no-matches", "--size", "1100x700"],
    "compare-unset": ["--fixture", "compare-unset", "--size", "1100x700"],
}

EXAMPLES = json.loads((ROOT / "examples/demo-requests.json").read_text(encoding="utf-8"))["cases"]
for example in EXAMPLES:
    name = "example-" + example["id"]
    CASES[name] = ["--fixture", name, "--size", "1100x700"]
CASES["example-box-overview"] = ["--fixture", "example-invoice-box", "--size", "1100x700", "--script", "tests/inspector-overview.script.json"]
CASES["example-noul-overview"] = ["--fixture", "example-fraud-check", "--size", "1100x700", "--script", "tests/inspector-overview.script.json"]

CASES["example-inventory-expanded"] = ["--fixture", "example-inventory", "--size", "1100x700", "--script", "tests/inspector-expand-json.script.json"]
CASES["example-box-light"] = ["--fixture", "example-invoice-box", "--size", "1100x700", "--script", "tests/inspector-overview-light.script.json", "--scroll-bars", "always"]

CASES["example-unicode-scrolled"] = ["--fixture", "example-unicode", "--size", "1100x700", "--script", "tests/inspector-unicode-scroll.script.json"]
CASES["example-many-choices-bottom"] = ["--fixture", "example-many-choices", "--size", "1100x700", "--script", "tests/inspector-diagram-bottom.script.json"]
CASES["example-many-choices-top"] = ["--fixture", "example-many-choices", "--size", "1100x700", "--script", "tests/inspector-diagram-top.script.json"]
CASES["example-choice-expanded"] = ["--fixture", "example-support-routing", "--size", "1100x700", "--script", "tests/inspector-diagram-expanded.script.json"]
CASES["example-diagram-keyboard"] = ["--fixture", "example-many-choices", "--size", "1100x700", "--script", "tests/inspector-diagram-keyboard.script.json"]
CASES["example-diagram-keyboard-state"] = ["--fixture", "example-many-choices", "--size", "1100x700", "--script", "tests/inspector-diagram-keyboard-state.script.json"]


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
    if name.startswith("example-"):
        if "showcase-tour" not in keyed:
            errors.append("Example did not reach the offline showcase")
        slug = CASES[name][1].removeprefix("example-")
        example_id = next(i + 1 for i, case in enumerate(EXAMPLES) if case["id"] == slug)
        if f"Exchange #{example_id:03}" not in names:
            errors.append("Example selected the wrong exchange")
        if slug in ("unicode", "inventory") and "selected" not in keyed.get("tab-1", {}).get("states", []):
            errors.append("Large/Unicode example must show its request payload")
    if name in ("example-many-choices-bottom", "example-many-choices-top"):
        expected = "Show top" if name.endswith("bottom") else "Show bottom"
        if keyed.get("request-diagram:card-reveal", {}).get("name") != expected:
            errors.append(f"Tall-card navigation did not reach {expected}")
    if name == "example-choice-expanded" and "requests" in keyed:
        errors.append("Expected expanded diagram")
    if name in ("example-diagram-keyboard", "example-diagram-keyboard-state"):
        target = "5:state" if name.endswith("-state") else "6:answer"
        control = keyed.get(f"request-diagram:node:{target}:select", {})
        if "focused" not in control.get("states", []):
            errors.append(f"Expected keyboard focus on {target}")
        r, v = control.get("bounds"), keyed.get("request-diagram:viewport", {}).get("bounds")
        if not r or not v or not (r[0] >= v[0] and r[1] >= v[1] and r[0]+r[2] <= v[0]+v[2]+0.5 and r[1]+r[3] <= v[1]+v[3]+0.5):
            errors.append("Keyboard focus is outside the visible diagram canvas")
        if name.endswith("-state") and "request-diagram:card-overflow" in keyed:
            errors.append("Overflow hint must describe the active card, not a different clipped card")
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
    if name.startswith("loading-"):
        if "detail-loading" not in keyed or "Loading request details" not in names:
            errors.append("Loading fixture did not reach the loading panel")
        if "loading-preview" in keyed and keyed["loading-preview"].get("children"):
            errors.append("Skeleton decoration should be hidden from accessibility")
    new_states = {"detail-failed": "retry-detail", "disconnected-empty": "detail-unavailable",
                  "pending-response": "response-pending", "empty-response": "body-empty",
                  "no-matches": "reset-filters", "compare-unset": "set-baseline"}
    if name in new_states and new_states[name] not in keyed:
        errors.append(f"Missing {new_states[name]}")
    new_text = {"first-use": "No requests yet", "saved-empty": "No requests in this session"}
    if name in new_text and new_text[name] not in names:
        errors.append(f"Missing {new_text[name]}")
    if name in ("compare-small-split", "compare-small-expanded") and keyed.get("diff-mode", {}).get("name") != "Unified":
        errors.append("Expected split comparison mode")
    if name == "compare-small-expanded" and "requests" in keyed:
        errors.append("Expected expanded comparison")
    required_text = {"saved-session": "SAVED WITH BODIES", "summary-only": "Summary only",
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
    with binary.open("rb") as stream:
        binary_sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    try:
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, stderr=subprocess.DEVNULL).strip()
        dirty = bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True))
    except (OSError, subprocess.CalledProcessError):
        revision, dirty = None, None
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
              "machine": platform.machine(), "binary": str(binary), "binary_sha256": binary_sha256,
              "source_revision": revision, "working_tree_dirty": dirty, "scale": args.scale,
              "measurement": "Forced headless frames, including scripted transitions. Not interactive FPS or network latency. Compare the same build profile and scale.",
              "results": results}
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return 0 if all(r["ok"] for r in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
