# Use the local Rune diagram components

The tracked implementation is `crates/rune-diagram`. The inspector depends on it
as a Rust library. `python tools/install_rune_diagram.py` installs a copy into
`.upstream/rune-inspector/crates/rune-diagram` and registers it in that prepared
Rune workspace, so other local Rune apps can use it with the focus patch.
This is a local extension; no upstream commit, push, or publication is required.

## Open a captured request diagram

Build and open the offline inspector fixture:

```sh
python tools/prepare_rune.py
cargo build --workspace --locked
cargo run -p jev-inspector -- --demo --diagram
```

For live capture, open the inspector normally and select **Diagram** beside
Overview, Request, and Response. Select a question to see its captured context,
question definition, returned answer, and probability distribution. The question
selector scrolls through long lists without reducing the canvas height; End and
Enter select its final option.
The field and menu size to the question labels. Short lists stay compact; long
names get a wider menu within the available space. Hover a shortened label to
read its full name.
Use `python dev.py demo --diagram` to start the synthetic traffic demo on that tab.
Graphs start fitted to the available canvas. **Actual size** restores 100% zoom;
**Fit graph** returns to the full graph. Manual pan and zoom survive resizing.
**Expand** hides the request list and counters to give the canvas more
space. Choice results have separate returned-choice and alternative cards; Noul
results show P(True) and its explicitly labeled complement, P(False). Neither
branch implies that a backend condition was executed. Large choice distributions
start with three leading categories plus the returned choice, and expand by three.
The diagram caps this preview at 120 leading categories plus the returned choice;
the Response tab retains the available captured JSON.

The diagram displays one HTTP exchange. Its connections show which captured data
belongs to that question. They do not represent internal model reasoning or a
multi-request backend workflow. Errors and missing capture data remain explicit.

## Open a workflow example

```sh
cargo run -p rune-diagram --example flow
```

This separate example demonstrates branching cards using an application-supplied
graph. It uses synthetic data and makes no requests.

## Update the copy in the local Rune checkout

After changing the tracked crate, run:

```sh
python tools/install_rune_diagram.py
```

On Windows, use `.venv\Scripts\python.exe` if Python is not on PATH.
The script requires an existing Rune checkout. `--checkout PATH` selects another
local checkout. It copies the source and license, translates dependencies to
local Rune paths, registers the workspace member, and adds a changelog entry.
It refuses to overwrite files edited in the installed copy. Move those changes
back into the tracked crate before reinstalling.

Inside the Rune checkout, run:

```sh
cargo test -p rune-diagram
cargo run -p rune-diagram --example flow
```

An app in that checkout can depend on the installed crate using a Cargo path
dependency. See [the component API](../crates/rune-diagram/README.md) and
[the workflow example](../crates/rune-diagram/examples/flow.rs).

## Verify changes

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run -p jev-inspector -- --demo --diagram --frames 3 --snap artifacts/inspector/diagram.png
```

Runtime tests cover camera fit/zoom, selection, JSON disclosure, wheel input, and
graph validation. The inspector adapter tests missing capture data, returned choice
versus alternative branches, Noul complements, and compact category expansion.
macOS native input still needs verification.
Agent launches on Windows must use the normal desktop outside the execution
sandbox; a sandbox process can create a window invisible to the user.

## Inspect card data

Select a card and choose **Inspect data** in the diagram toolbar, or double-click
its heading. The reader opens inside the canvas and keeps a fixed text size while
the graph stays at its current zoom. Use **JSON** for highlighted, wrapped content
or **Tree** for nested properties. Large JSON values have page controls; **Copy**
copies the full value, including content outside the current page or card preview.

Escape or Close dismisses the reader and returns focus to Inspect data. Selecting
a different request or question clears the previous reader. The surrounding app
remains interactive.

Reusable graphs can call `Node::inspection(Some(value))` to supply the full value
behind a summarized card; otherwise the reader uses its `NodeBody`.
