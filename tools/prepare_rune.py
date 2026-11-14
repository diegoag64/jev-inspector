"""Prepare the inspector's pinned Rune source and tracked local focus patch."""
from pathlib import Path
import hashlib
import io
import json
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / ".upstream" / "rune-inspector"
PATCH = ROOT / "patches" / "rune-focus.patch"
REVISION = "d77610b546ce70ca3d8ffcb22e078278d2bb79b7"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare():
    receipt = TARGET / ".inspector-patch.json"
    if receipt.exists():
        saved = json.loads(receipt.read_text())
        if saved["revision"] != REVISION or saved["patch"] != digest(PATCH):
            raise RuntimeError("Rune patch changed; preserve local edits before preparing a new source directory.")
        if any(digest(TARGET / name) != value for name, value in saved["files"].items()):
            raise RuntimeError("Patched Rune files contain local edits; refusing to overwrite them.")
        return
    if TARGET.exists():
        raise RuntimeError(f"Unmanaged directory exists: {TARGET}")
    candidates = [ROOT / ".upstream" / "rune"]
    candidates += list((Path.home() / ".cargo" / "git" / "checkouts").glob(f"rune-*/{REVISION[:7]}"))
    source = next((p for p in candidates if (p / ".git").exists()), None)
    if source is None:
        source = ROOT / ".upstream" / "rune-source"
        if not (source / ".git").exists():
            subprocess.run(["git", "clone", "--no-checkout", "--filter=blob:none",
                            "https://github.com/anomalyco/rune", str(source)], check=True)
        subprocess.run(["git", "-C", str(source), "fetch", "origin", REVISION], check=True)
    # Read the exact pinned object from this cache, even when a sandbox account
    # differs from the cache owner. Do not change the user's global Git config.
    archive = subprocess.check_output(["git", "-c", f"safe.directory={source.resolve().as_posix()}",
                                       "-C", str(source), "archive", REVISION])
    TARGET.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        tar.extractall(TARGET, filter="data")
    subprocess.run(["git", "apply", f"--directory={TARGET.relative_to(ROOT).as_posix()}",
                    str(PATCH)], cwd=ROOT, check=True)
    names = [line[6:] for line in PATCH.read_text().splitlines() if line.startswith("+++ b/")]
    receipt.write_text(json.dumps({"revision": REVISION, "patch": digest(PATCH),
                                  "files": {name: digest(TARGET / name) for name in names}}, indent=2) + "\n")
    print("Prepared pinned Rune with the inspector focus patch.")


if __name__ == "__main__":
    prepare()
