"""Shared synthetic HTTP examples for the relay demo and offline Rust showcase."""
from __future__ import annotations

import json
from pathlib import Path

CATALOG = Path(__file__).resolve().parents[1] / "examples" / "demo-requests.json"


def load_cases() -> list[dict]:
    document = json.loads(CATALOG.read_text(encoding="utf-8"))
    if document["version"] != 1:
        raise ValueError("Unsupported demo catalog version")
    cases = document["cases"]
    if len({case["id"] for case in cases}) != len(cases):
        raise ValueError("Duplicate demo case IDs")
    return cases


def response_bytes(case: dict) -> bytes:
    if "response_text" in case:
        return case["response_text"].encode("utf-8")
    if case.get("response") is None:
        return b""
    return json.dumps(case["response"], ensure_ascii=False, separators=(",", ":")).encode("utf-8")
