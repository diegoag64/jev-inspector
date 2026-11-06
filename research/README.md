# Run the bridge compatibility experiment

This experiment sends synthetic requests from the official TypeSafe JavaScript and
Python SDKs through a local HTTP relay to a local mock service. It makes no TypeSafe
API calls and uses a fake key. Both servers bind ephemeral IPv4 loopback ports and
close after the run.

The relay buffers bodies up to 1 MiB. It is research code, not the proposed Rust
implementation, a production proxy, or a performance benchmark. It tests the
integration point and selected HTTP behavior.

## Requirements

- Node 22.15 or later with zstd support.
- Python 3.10 or later.
- The published JavaScript package `@typesafe-ai/sdk@0.7.0`.
- The Python package `typesafe-sdk[zstd]==0.7.5` in an isolated environment.

## Run on Windows

From the repository root, with Node, npm, and Python on PATH:

```powershell
python -m venv .upstream/research-venv
& .upstream/research-venv/Scripts/python.exe -m pip install 'typesafe-sdk[zstd]==0.7.5'
npm install --prefix .upstream/bridge-js --ignore-scripts --no-audit --no-fund '@typesafe-ai/sdk@0.7.0'
node research/run-bridge-probe.mjs .upstream/bridge-js/node_modules/@typesafe-ai/sdk/dist/index.mjs .upstream/research-venv/Scripts/python.exe
```

## Run on macOS or Linux

```sh
python3 -m venv .upstream/research-venv
.upstream/research-venv/bin/python -m pip install 'typesafe-sdk[zstd]==0.7.5'
npm install --prefix .upstream/bridge-js --ignore-scripts --no-audit --no-fund '@typesafe-ai/sdk@0.7.0'
node research/run-bridge-probe.mjs .upstream/bridge-js/node_modules/@typesafe-ai/sdk/dist/index.mjs .upstream/research-venv/bin/python
```

These setup commands are provided for reuse. This research run used the tool's Node
execution environment, the downloaded npm package, and a Python environment created
with uv. The command-line wrapper and macOS/Linux commands have not been run here.

## Read the result

Success prints JSON with eight JavaScript scenarios, nine Python scenarios, and 23
HTTP requests. Retries account for the extra requests. A failure throws an assertion
or an SDK error and the command-line wrapper exits nonzero.

The probe checks:

- Explicit JavaScript `baseURL` and Python `TYPESAFE_BASE_URL` routing.
- JSON requests, multipart image bytes and the image opt-in header.
- Large requests and zstd when the JavaScript runtime exposes Node compression.
  Python must send zstd for its compressed case.
- Gzip responses, typed answers, and `x-typesafe-request-id` access.
- `GET /v1/models`, a non-retried 401, a 529 followed by success, and three
  attempts for a persistent 429 with SDK defaults.
- Equal request and response body SHA-256 values at the bridge and mock service.
- A Python asynchronous client in addition to synchronous calls.

The completed Windows run passed all 17 scenarios. JavaScript reported
`javascriptCompression: false` because the tool's execution environment does not
expose normal Node process detection to the SDK. Its large request was uncompressed.
Python zstd was exercised. Normal Node automatic zstd selection remains to be tested
through the command-line runner.

The local result is `artifacts/research/bridge-probe.json`. Artifacts and dependency
downloads are ignored by Git. The feasibility assessment is
[docs/jev-bridge-feasibility.md](../docs/jev-bridge-feasibility.md).

## Test the implemented Rust relay

Build with `cargo build -p jev-bridge --locked`, then append the executable path
and a new connection-file path to the probe command. On Windows:

```powershell
node research/run-bridge-probe.mjs .upstream/bridge-js/node_modules/@typesafe-ai/sdk/dist/index.mjs .upstream/research-venv/Scripts/python.exe target/debug/jev-bridge.exe .jev/sdk-probe.json
```

On macOS, use the Python environment's `bin/python` and `target/debug/jev-bridge`.
The connection-file path must not already exist. The probe starts the Rust process
on ephemeral loopback ports and stops it after testing. The temporary connection
file is removed on normal completion.

In this mode, the test-side observer forwards through the real Rust relay to the
mock service. The probe verifies byte hashes and the inspection API's retained
attempts, compressed JSON, multipart request JSON and omission of the API key.
All 17 scenarios passed locally, with 23 HTTP exchanges. The evidence is
`artifacts/research/rust-bridge-probe.json`. The JavaScript runtime limitation
described above still applies to that local run.
