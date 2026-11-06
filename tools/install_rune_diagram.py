"""Install the tracked rune-diagram crate into the pinned local Rune checkout."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def install(checkout):
    checkout = checkout.resolve()
    workspace = checkout / "Cargo.toml"
    source = ROOT / "crates" / "rune-diagram"
    target = checkout / "crates" / "rune-diagram"
    if not (checkout / "crates" / "rune" / "Cargo.toml").is_file():
        raise SystemExit("Expected a Rune checkout with crates/rune/Cargo.toml.")
    files = {p.relative_to(source).as_posix(): p.read_bytes()
             for p in source.rglob("*") if p.is_file() and "target" not in p.parts}
    manifest = files["Cargo.toml"].decode().replace(
        "rune.workspace = true", 'rune = { path = "../rune", default-features = false, features = ["accessibility", "embedded-fonts", "metal-backend"] }'
    ).replace("rune-kit.workspace = true", 'rune-kit = { path = "../rune-kit" }')
    files["Cargo.toml"] = manifest.encode()
    receipt = target / ".local-install.json"
    previous = json.loads(receipt.read_text()) if receipt.exists() else {}
    # Refuse to overwrite local Rune edits. Identical source can be installed again.
    for name, data in files.items():
        path = target / name
        if path.exists() and path.read_bytes() != data and digest(path.read_bytes()) != previous.get(name):
            raise SystemExit(f"Local edits in {path}. Copy them back to crates/rune-diagram before installing.")
    original = workspace.read_text(encoding="utf-8")
    member = '    "crates/rune-diagram",'
    updated = original
    if '"crates/rune-diagram"' not in original:
        marker = '    "crates/rune-kit",'
        if marker not in original:
            raise SystemExit("Rune workspace member list was not recognized; no files changed.")
        updated = original.replace(marker, marker + "\n" + member, 1)
    for name, data in files.items():
        path = target / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    receipt.write_text(json.dumps({name:digest(data) for name,data in files.items()}, indent=2)+"\n")
    ignore = target / ".gitignore"
    if not ignore.exists():
        ignore.write_text(".local-install.json\n")
    if updated != original:
        workspace.write_text(updated, encoding="utf-8", newline="\n")
    changelog = checkout / "changelog" / "unreleased" / "2026-11-06-diagram-components.md"
    if not changelog.exists():
        changelog.parent.mkdir(parents=True, exist_ok=True)
        changelog.write_text("- Add a local `rune-diagram` crate with interactive connected cards, camera controls, bounded JSON disclosures, probability bars, and a synthetic workflow example adapted from jev-devtools.\n")
    print(f"Installed rune-diagram into {target}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", type=Path, default=ROOT / ".upstream" / "rune")
    install(parser.parse_args().checkout)
