# Prepare the inspector's local Rune patch

Run this once before building on Windows or macOS:

```sh
python tools/prepare_rune.py
cargo build --workspace --locked
```

The script exports Rune revision `781e2b6d87cd5f32d6f4365956fad4baaf24ee50`
to `.upstream/rune-inspector` and applies `patches/rune-focus.patch`. It reuses
the pinned QA checkout or Cargo cache when available; otherwise it downloads
the pinned repository. The original `.upstream/rune` gallery source is unchanged.
Cargo patches all five Rune packages to the prepared workspace to keep their
shared types consistent. The exported sources are ignored by Git; the patch,
setup script, and dependency lockfile are tracked.

`python dev.py demo` prepares the source automatically. To install the reusable
diagram crate into this workspace, run `python tools/install_rune_diagram.py`.
Other Rune workspaces need the same focus patch before using this version of
the diagram crate. No upstream publication is involved.

The patch adds `Tx::focus_control(key)` for a currently mounted control. It uses
Rune's existing focus eligibility checks, respects modal boundaries, and does
not activate the target or scroll a different pane. Missing or unreachable
targets leave focus unchanged. The last focus or blur request wins. The existing
keyboard-versus-pointer focus-ring behavior is preserved.

The inspector uses this API when paging would hide a focused property, and when
pressing a property's scrollbar. Tests cover focus ownership, modal containment,
missing targets, request ordering, nested disclosures, pointer/keyboard switching,
and thumb displacement at independent diagram zoom and renderer scales.

Repeated preparation verifies the patched files against its receipt and refuses
to overwrite local changes. Preserve any edits before changing the patch or
replacing the generated source directory.
