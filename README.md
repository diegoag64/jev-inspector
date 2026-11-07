# Jev inspector

A native desktop inspector for TypeSafe API traffic, built in Rust with [Rune](https://github.com/anomalyco/rune). Point a backend at the local relay to inspect its requests, responses, and Jev answers without adding a React devtools panel to the application.

```text
Your backend -> jev-bridge -> TypeSafe API
                    |
               jev-inspector
```

The relay forwards requests and keeps a bounded capture history. The inspector reads that history through a separate local API. Closing the inspector does not stop an independently launched relay.

## Try it locally

Install Git, Python 3.11 or later, and Rust through [rustup](https://rustup.rs/). On Windows, install Visual Studio Build Tools with **Desktop development with C++** and a Windows SDK. On macOS, install Xcode Command Line Tools.

Clone this repository and run from its root:

```sh
git clone https://github.com/diegoag64/rune-qa.git
cd rune-qa
python dev.py demo
```

Use `python3` on macOS if needed. The launcher prepares the pinned Rune sources, builds the Rust binaries, starts a mock API and relay, and opens the inspector. The first build takes longer while Cargo downloads and compiles dependencies.

The demo sends one batch of varied synthetic requests, then stays idle. It needs no API key and does not contact TypeSafe. Close the window to stop the demo and its helper processes. Add `--continuous` to repeat the batch, or `--skip-build` when the binaries are current.

For offline examples without a relay:

```sh
python tools/prepare_rune.py
cargo run -p jev-inspector -- --showcase
```

To inspect your own backend, follow the [bridge quickstart](docs/bridge-quickstart.md). It covers relay ports, body capture, and the client's base URL.

## Inspect an exchange

- Filter and sort traffic, star requests, pause updates, or follow incoming traffic.
- Read formatted JSON, search captured bodies, and expand nested values in the property tree.
- Explore Jev answers as connected cards with probabilities and input properties.
- Compare an exchange with a baseline, including warnings for incomplete captures.
- Export a session, CSV summaries, or one exchange. Reopen sessions offline.

See the [workspace guide](docs/inspector-workspace.md) for controls, keyboard shortcuts, and capture limits.

## Project layout

| Path | Purpose |
| --- | --- |
| `crates/jev-inspector` | Rune desktop UI |
| `crates/jev-bridge` | Local HTTP relay and capture API |
| `crates/jev-protocol` | Shared exchange and session types |
| `crates/rune-diagram` | Reusable graph and answer-card components |
| `examples/demo-requests.json` | Synthetic request and response catalog |
| `dev.py` | Local demo launcher |
| `qa.py` | Upstream Rune gallery checks and performance comparisons |
| `tools/` | Source preparation and inspector QA scripts |

The inspector uses a pinned Rune revision with a small [focus patch](docs/rune-focus-patch.md). Downloaded sources, binaries, captures, and local connection files are ignored by Git. Keep reusable changes in this repository, not in `.upstream`.

## Development and QA

Start with [CONTRIBUTING.md](CONTRIBUTING.md) for build commands, checks, and commit conventions. Checks run locally; this repository has no CI/CD workflows.

- [Inspector examples and motion checks](docs/inspector-examples.md)
- [Reusable diagram components](docs/diagrams.md)
- [Rune component QA and cross-machine comparisons](docs/component-qa.md)
- [Relay behavior and implementation limits](docs/bridge-v1.md)
- [Native Windows checks](docs/native-windows-check.md)

This is an experimental local development tool. Captured bodies can contain application data even after field-name redaction, so review exports before sharing them. Automated headless checks cover layout and application behavior; they do not certify native clipboard, file dialogs, screen-reader speech, or macOS behavior. CPU frame timings are not GPU execution times.
