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

The patch adds `Tx::focus_control(key)`, resolved after the view rebuilds and
before the next queued keyboard event. A control enabled or mounted by the same
interaction can receive focus. It uses
Rune's existing focus eligibility checks, respects modal boundaries, and does
not activate the target or scroll a different pane. Missing or unreachable
targets leave focus unchanged. The last focus or blur request wins, including
separate host updates queued before a frame. A later blur cancels pending focus.
The existing
keyboard-versus-pointer focus-ring behavior is preserved.

The inspector uses this API when paging would hide a focused property, and when
pressing a property's scrollbar. An inactive scrollbar activates its owning card
without reframing the camera. Tests cover focus ownership, modal containment,
missing targets, request ordering, nested disclosures, pointer/keyboard switching,
and thumb displacement at independent diagram zoom and renderer scales.

Repeated preparation verifies the patched files against its receipt and refuses
to overwrite local changes. Preserve any edits before changing the patch or
replacing the generated source directory.

The patch also updates RuneKit Select labels to retain the beginning and end of
long option names. Options expose their full names as wrapped hover help, while
accessible names retain the original text. Diagram headings use the same visual
truncation and full-name help in the reusable diagram crate.
