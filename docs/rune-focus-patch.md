# Prepare the inspector's local Rune patch

For a new checkout, run this before building on Windows or macOS. Use Python
3.11.4 or later. For an existing prepared source, follow
[the upgrade procedure](#update-an-existing-prepared-checkout).

```sh
python tools/prepare_rune.py
cargo build --workspace --locked
```

The script exports Rune revision `d77610b546ce70ca3d8ffcb22e078278d2bb79b7`
to `.upstream/rune-inspector` and applies `patches/rune-focus.patch`. It reuses
the pinned QA checkout or Cargo cache when available; otherwise it downloads
the pinned repository. The original `.upstream/rune` gallery source is unchanged.
Cargo redirects five git packages (`rune`, `rune-kit`, `rune-motion`,
`rune-staging`, and `rune-taffy`) to the prepared workspace. Other local
dependencies, including `rune-highlights` and `tiny-skia-path`, are resolved
through path dependencies inside that workspace. The root Cargo manifest also
repeats Rune's required vendored patches for `wgpu-hal`, `winit`, `parley_engine`, and
`accesskit_windows`; Cargo does not inherit dependency workspace patches.
The exported sources are ignored by Git; the patch, setup script, and dependency
lockfile are tracked.

`python dev.py demo` prepares the source automatically. To install the reusable
diagram crate into this workspace, run `python tools/install_rune_diagram.py`.
Other Rune workspaces need the same focus patch before using this version of
the diagram crate. No upstream publication is involved.

The patch adds `Tx::focus_control(key)`, resolved after the view rebuilds and
before the next queued keyboard event. A control enabled or mounted by the same
interaction can receive focus. It uses Rune's existing focus eligibility checks,
respects modal boundaries, and does not activate the target or scroll a different pane. Missing or unreachable
targets leave focus unchanged. The last focus or blur request wins, including
separate host updates queued before a frame. A later blur cancels pending focus.
The existing keyboard-versus-pointer focus-ring behavior is preserved.

The inspector uses this API when paging would hide a focused property, and when
pressing a property's scrollbar. An inactive scrollbar activates its owning card
without reframing the camera. Tests cover focus ownership, modal containment,
missing targets, request ordering, nested disclosures, pointer/keyboard switching,
and thumb displacement at independent diagram zoom and renderer scales.

Repeated preparation checks the revision, patch digest, and hashes of the files
listed in `.inspector-patch.json`. It refuses a changed receipt or changes to
those patched files. It does not verify every exported file. Preserve any edits
before changing the patch or replacing the generated source directory.

The patch also updates RuneKit Select labels to retain the beginning and end of
long option names. Options expose their full names as wrapped hover help, while
accessible names retain the original text. Diagram headings use the same visual
truncation and full-name help in the reusable diagram crate.

`Table::decorate_row` lets the inspector color each arriving request across the
whole row. The callback keeps the table's cells, selection handlers, and virtual
row identity. It must preserve row height so scrolling geometry stays correct.

## Update an existing prepared checkout

When the pinned revision or patch changes, preserve the existing generated
directory before preparing the new source. Choose an unused backup name before
running these commands. The Python rename works in both PowerShell and a POSIX shell.

```sh
python -c "from pathlib import Path; Path('.upstream/rune-inspector').rename('.upstream/rune-inspector-backup')"
python tools/prepare_rune.py
cargo clean
cargo build --workspace --locked
```

Retain the backup until you have recovered any local edits and verified the new
build. If you installed `rune-diagram` into the exported workspace, rerun
`python tools/install_rune_diagram.py` after preparation.

Clear the Rune build cache after replacing sources. Git archives preserve source
timestamps, so Cargo can otherwise reuse artifacts from the previous revision.
`cargo clean` clears the root workspace's build artifacts, including transitive
Rune and vendored packages. The next build recompiles dependencies. It does not
remove source backups or clean the separate gallery target directory.

If preparation selects `.upstream/rune` but reports that the revision is missing,
fetch the pinned object into that cache before retrying. This updates Git objects
without checking out files or changing local source edits:

```sh
git -C .upstream/rune fetch origin d77610b546ce70ca3d8ffcb22e078278d2bb79b7
python tools/prepare_rune.py
```

The gallery runner has its own setup and build workflow. Follow the
[component QA guide](component-qa.md) to update that checkout and its provenance.
