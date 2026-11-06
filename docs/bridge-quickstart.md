# Run the Jev bridge and inspector

The first version has two independent Rust programs. `jev-bridge` forwards TypeSafe
HTTP calls and keeps a bounded history in memory. `jev-inspector` displays that
history in a native Rune window. The existing component QA runner remains available
as `qa.py`.

## Build

Install Git and Rust. The validated compiler is Rust 1.96.0. Windows needs Visual
Studio C++ Build Tools and a Windows SDK. macOS needs Xcode Command Line Tools.

From the repository root, run:

```sh
cargo build --workspace --locked
```

Cargo downloads the pinned Rune revision and dependencies. The binaries are in
`target/debug`. The bridge can also be built independently with
`cargo build -p jev-bridge --locked` and does not link Rune or require a GPU.

## Try synthetic traffic

Run:

```sh
python dev.py demo
```

On this Windows workspace, use `.venv\Scripts\python.exe` if `python` is not on PATH.
On macOS, use `python3` if needed. The script uses only Python's standard library.

The command builds the binaries, starts a loopback mock API, starts the real Rust
relay, generates synthetic requests, and opens the inspector. It does not contact
TypeSafe or use a real API key. The mock intentionally produces some 401 and 429
responses so the error filter has data. The catalog also includes mixed answer
types, nested and multilingual input, an empty response, non-JSON output, and a
slow request. It sends one batch of 14 requests, then stops generating traffic.
Add `--continuous` to keep generating requests. For offline layout playback, see
[the example tour](inspector-examples.md).

Select a row, open its Request and Response tabs, search for `429`, and switch
between light and dark themes. The Overview tab shows returned answer types,
probabilities, usage and timing. Timing includes network and upstream work.

Request and Response show indented JSON with Rune's built-in syntax highlighting,
line numbers, and horizontal and vertical scrolling. Drag over the code to select
text, or use **Copy JSON** for the complete captured JSON, including content beyond
the display preview limit. Copies contain the inspection data after capture limits
and redaction, not the original wire bytes. Capture notes remain outside the code.

Use **Pause**, **Follow**, filters, sortable columns, and stars to inspect live
traffic. **Tree** explores JSON properties, and **Compare** shows changes between
two exchanges. **Export** saves a session, CSV summaries, or one exchange. **Open**
loads a saved session without mixing it with incoming traffic. See
[Inspect and save requests](inspector-workspace.md) for controls, shortcuts, limits,
and the reusable screenshot runner.

Close the window to stop this demo and its helper processes. This shutdown behavior
belongs to the demo launcher. An independently launched relay keeps running when
the inspector closes.

If the binaries are already current, use `python dev.py demo --skip-build`.
If the default ports are occupied, add `--port 8797 --inspector-port 8798`.
Logs are in `artifacts/demo`. The demo connection file is `.jev/demo-connection.json`.

When an agent launches this demo on Windows, it must launch on the user's normal
desktop outside its execution sandbox. A sandboxed process can report a window
title and initialize the GPU while its window remains invisible to the user.
Running the command yourself from a normal terminal avoids this issue.

For a static UI fixture without any servers, run:

```sh
cargo run -p jev-inspector -- --demo
```

## Connect an actual backend

Start the relay in one terminal:

```sh
cargo run -p jev-bridge -- --capture-bodies
```

The default upstream is `https://api.typesafe.ai`. Body capture keeps JSON inspection
copies in memory. Omit `--capture-bodies` for metadata-only capture. Sensitive prose
can remain in captured JSON; the built-in redaction covers common field names,
not arbitrary application secrets.

Start the inspector in a second terminal from the same repository root:

```sh
cargo run -p jev-inspector
```

In the terminal that launches your backend, set the TypeSafe base URL.

Windows PowerShell:

```powershell
$env:TYPESAFE_BASE_URL = "http://127.0.0.1:8787"
# Start your backend using its existing command and existing TypeSafe API key.
```

macOS:

```sh
export TYPESAFE_BASE_URL=http://127.0.0.1:8787
# Start your backend using its existing command and existing TypeSafe API key.
```

An explicit JavaScript `baseURL` or Python `base_url` setting overrides the
environment variable. Use the origin above without `/v1`; the SDK appends the
endpoint path. Actual backend calls still reach TypeSafe and use its normal billing.

The inspector reconnects after the relay restarts. Restarting clears history.
Closing the inspector leaves this separately launched relay running. Ctrl+C in the
relay terminal stops forwarding. Restore the original base URL before stopping a
relay that your backend still depends on.

## Change local ports or configuration paths

```sh
cargo run -p jev-bridge -- --listen 127.0.0.1:8797 --inspector 127.0.0.1:8798 --connection-file .jev/other.json --capture-bodies
cargo run -p jev-inspector -- --connection-file .jev/other.json
```

Point the backend at the chosen relay port. The connection file contains a random
token for the inspection API. `.jev/` is ignored by Git. Keep the file private and
do not put it in a shared directory. On Unix it is created with mode 0600; on Windows
it inherits the parent directory's permissions.

Version 1 accepts loopback listeners only. Docker containers, WSL environments and
remote servers need a deliberately configured network arrangement; their localhost
does not necessarily refer to the host running this app. A shared LAN portal is not
implemented in this version.

## Verify changes

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

These tests cover body preservation, retries, capture limits, redaction, access
controls, response streaming, disconnects, parallel requests and Rune interactions.
For official SDK contract tests, follow [the probe instructions](../research/README.md).

To capture the static inspector fixture on Windows:

```powershell
New-Item -ItemType Directory -Force artifacts/inspector | Out-Null
target/debug/jev-inspector.exe --demo --scale 1 --snap artifacts/inspector/demo.png --tree artifacts/inspector/demo.json
target/debug/jev-inspector.exe --demo --scale 1 --script tests/inspector-request.script.json --at 0.8 --snap artifacts/inspector/request-light.png --tree artifacts/inspector/request-light.json
```

On macOS, create the output directory with `mkdir -p artifacts/inspector` and use
`target/debug/jev-inspector` without `.exe`.

For performance work, build with `cargo build --workspace --profile perf --locked`.
The demo accepts `--profile perf`. Keep fixture size, scale, app revision, SDK
versions and build profile equal across machines. The current validation does not
establish a relay latency budget or an operating-system performance comparison.

See [the implementation contract and validation record](bridge-v1.md) for limits
and the remaining platform checks.
