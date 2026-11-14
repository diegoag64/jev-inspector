"""Package Jev Inspector as a double-clickable macOS app.

    python3 tools/bundle_macos.py            # dist/Jev Inspector.app
    python3 tools/bundle_macos.py --install  # also copy it to /Applications
    python3 tools/bundle_macos.py --dmg      # also build dist/Jev-Inspector.dmg

The app hosts the relay itself, on the port chosen during onboarding, so it needs
no terminal. The bundle is signed ad hoc: it opens on the Mac that built it. Sharing
it with other Macs needs a Developer ID signature and notarization.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
APP_NAME = "Jev Inspector"
BUNDLE_ID = "dev.jev.inspector"  # Matches the preferences folder the inspector already uses.
EXECUTABLE = "jev-inspector"
PINK, VIOLET = (0xE5, 0x51, 0xBA), (0x7C, 0x5C, 0xFF)


def workspace_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"', text, re.M | re.S)
    if not match:
        raise SystemExit("Cannot read the workspace version from Cargo.toml.")
    return match.group(1)


def info_plist(version: str) -> dict:
    return {
        "CFBundleDevelopmentRegion": "en",
        "CFBundleDisplayName": APP_NAME,
        "CFBundleExecutable": EXECUTABLE,
        "CFBundleIconFile": "AppIcon",
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": APP_NAME,
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": version,
        "LSApplicationCategoryType": "public.app-category.developer-tools",
        "LSMinimumSystemVersion": "13.0",
        "NSHighResolutionCapable": True,
        "NSSupportsAutomaticGraphicsSwitching": True,
        "NSHumanReadableCopyright": "MIT License. Not affiliated with TypeSafe AI.",
    }


def draw_icon(path: Path) -> None:
    """The app mark: a source, the relay ring, and the upstream on a pink-to-violet tile."""
    from PIL import Image, ImageChops, ImageDraw, ImageFilter

    scale = 2  # Supersample, then downsample for smooth edges.
    size = 1024 * scale
    inset, radius = 100 * scale, 185 * scale  # Apple's icon grid: an 824 pt body in 1024.
    box = (inset, inset, size - inset, size - inset)

    # A diagonal gradient, drawn small and scaled up (it has no detail to lose).
    small = 256
    gradient = Image.new("RGBA", (small, small))
    pixels = gradient.load()
    for y in range(small):
        for x in range(small):
            t = (x + y) / (2 * (small - 1))
            pixels[x, y] = tuple(round(a + (b - a) * t) for a, b in zip(PINK, VIOLET)) + (255,)
    gradient = gradient.resize((size, size), Image.BILINEAR)
    mask = Image.new("L", (size, size))
    ImageDraw.Draw(mask).rounded_rectangle(box, radius, fill=255)

    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    shadow_mask = Image.new("L", (size, size))
    ImageDraw.Draw(shadow_mask).rounded_rectangle((box[0], box[1] + 18 * scale, box[2], box[3] + 18 * scale), radius, fill=110)
    shadow.putalpha(shadow_mask.filter(ImageFilter.GaussianBlur(28 * scale)))

    icon = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    icon.alpha_composite(shadow)
    tile = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    tile.paste(gradient, mask=mask)
    # A soft top highlight gives the tile some depth.
    highlight = Image.new("L", (size, size))
    ImageDraw.Draw(highlight).ellipse((-size * 0.2, -size * 0.75, size * 1.2, size * 0.4), fill=22)
    highlight = ImageChops.multiply(highlight.filter(ImageFilter.GaussianBlur(60 * scale)), mask)
    tile.alpha_composite(Image.merge("RGBA", (*[Image.new("L", (size, size), 255)] * 3, highlight)))
    icon.alpha_composite(tile)

    draw = ImageDraw.Draw(icon)
    white = (255, 255, 255, 255)
    cx = cy = size / 2
    body = box[2] - box[0]
    stroke = body * 0.042
    dot = body * 0.095 / 2
    ring = body * 0.3 / 2
    wire = body * 0.1
    left = cx - ring - wire - dot
    right = cx + ring + wire + dot
    draw.rectangle((left, cy - stroke / 2, cx - ring, cy + stroke / 2), fill=(255, 255, 255, 215))
    draw.rectangle((cx + ring, cy - stroke / 2, right, cy + stroke / 2), fill=(255, 255, 255, 215))
    for x in (left - dot, right + dot):
        draw.ellipse((x - dot, cy - dot, x + dot, cy + dot), fill=white)
    draw.ellipse((cx - ring, cy - ring, cx + ring, cy + ring), outline=white, width=round(stroke))
    inner = ring * 0.32
    draw.ellipse((cx - inner, cy - inner, cx + inner, cy + inner), fill=white)

    icon = icon.resize((1024, 1024), Image.LANCZOS)
    icon.save(path, sizes=[(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512), (1024, 1024)])


def build(profile: str) -> Path:
    from dev import find_cargo
    from tools.prepare_rune import prepare

    prepare()
    command = [find_cargo(), "build", "--locked", "-p", "jev-inspector", "--profile", profile]
    if subprocess.run(command, cwd=ROOT).returncode:
        raise SystemExit("The build failed; see the cargo output above.")
    binary = ROOT / "target" / profile / EXECUTABLE
    if not binary.is_file():
        raise SystemExit(f"Build finished without {binary}.")
    return binary


def bundle(binary: Path, out: Path) -> Path:
    app = out / f"{APP_NAME}.app"
    if app.exists():
        shutil.rmtree(app)
    macos = app / "Contents" / "MacOS"
    resources = app / "Contents" / "Resources"
    macos.mkdir(parents=True)
    resources.mkdir(parents=True)
    shutil.copy2(binary, macos / EXECUTABLE)
    with (app / "Contents" / "Info.plist").open("wb") as stream:
        plistlib.dump(info_plist(workspace_version()), stream)
    (app / "Contents" / "PkgInfo").write_text("APPL????", encoding="ascii")
    draw_icon(resources / "AppIcon.icns")
    # Apple Silicon refuses unsigned code. An ad hoc signature runs on this Mac.
    subprocess.run(["codesign", "--force", "--deep", "--sign", "-", str(app)], check=True)
    return app


def disk_image(app: Path, out: Path) -> Path:
    image = out / "Jev-Inspector.dmg"
    with tempfile.TemporaryDirectory() as staging:
        shutil.copytree(app, Path(staging) / app.name, symlinks=True)
        os.symlink("/Applications", Path(staging) / "Applications")
        image.unlink(missing_ok=True)
        subprocess.run(["hdiutil", "create", "-quiet", "-volname", APP_NAME, "-srcfolder", staging,
                        "-ov", "-format", "UDZO", str(image)], check=True)
    return image


def install(app: Path) -> Path:
    target = Path("/Applications") / app.name
    if target.exists():
        shutil.rmtree(target)
    subprocess.run(["ditto", str(app), str(target)], check=True)
    return target


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--profile", default="release", help="Cargo profile to build (default: release)")
    parser.add_argument("--out", type=Path, default=ROOT / "dist", help="Output folder (default: dist)")
    parser.add_argument("--install", action="store_true", help="Copy the app to /Applications")
    parser.add_argument("--dmg", action="store_true", help="Also create a drag-to-install disk image")
    parser.add_argument("--open", action="store_true", help="Open the app when done")
    args = parser.parse_args()
    if sys.platform != "darwin":
        raise SystemExit("Building a .app requires macOS.")
    try:
        import PIL  # noqa: F401
    except ImportError:
        raise SystemExit("The app icon needs Pillow: python3 -m pip install -r requirements.txt")
    args.out.mkdir(parents=True, exist_ok=True)
    app = bundle(build(args.profile), args.out.resolve())
    print(f"Built {app}")
    if args.dmg:
        print(f"Built {disk_image(app, args.out.resolve())}")
    if args.install:
        app = install(app)
        print(f"Installed {app}")
    if args.open:
        subprocess.run(["open", str(app)], check=True)


if __name__ == "__main__":
    main()
