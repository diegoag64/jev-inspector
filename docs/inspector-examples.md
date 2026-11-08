# Explore example requests and motion

The demo opens 10 synthetic Jev examples from
[`examples/demo-requests.json`](../examples/demo-requests.json). The Rust offline
showcase and Python mock server read the same catalog.

## Browse offline

```sh
cargo run -p jev-inspector -- --showcase
```

Select a request to inspect it. There is no Tour control in the app. The `--tour`
flag remains available for scripted animation checks; selecting a request or tab,
or opening Help or Export, stops playback.

The catalog covers choice probabilities, likelihoods, numeric scores, normalized
bounding boxes, mixed questions, twelve categories, a large nested inventory,
multilingual input, and a slow analysis request. The examples include customer
conversations, payment evidence, incident timelines, return policies, OCR blocks,
and warehouse movements. Nine requests succeed; one demonstrates rate limiting.

Four additional protocol fixtures (`models`, `auth-error`, `empty`, and `non-json`)
are marked `qa_only` in the catalog. They remain available through
`--fixture example-NAME` and the relay integration tests, but are not sent by the
normal demo or shown by `--showcase`.

Select a diagram card and choose **Inspect data**, or double-click its heading.
The reader stays inside the canvas at a fixed text size. JSON view pages large
values without dropping content; Tree provides nested disclosure. Copy always
includes the full captured value. Escape or Close returns to the diagram without
changing its zoom. The request list remains usable while the reader is open.

Offline examples are completed captures. To watch an exchange while its response
is pending, use the live demo below. Bounding-box previews show coordinates without
inventing or retaining an original document image.

## Send examples through the relay

```sh
python dev.py demo
```

On this Windows workspace, use `.venv\Scripts\python.exe` instead of `python`.
The launcher starts a local mock API and the actual Rust relay. It sends the 14
requests once, then waits with the inspector open. The slow request waits 2.4
seconds before returning. No TypeSafe account, real API key, or external request
is used. Close the inspector to stop the owned helper processes.

Add `--continuous` for repeating traffic, or `--skip-build` when the binaries are
current. The tour changes the selected request; continuous traffic creates new
requests. These are separate controls.

## Check transitions and compare machines

```sh
cargo build --workspace --locked
python tools/inspector_qa.py
python tools/inspector_motion.py
cargo test --workspace --locked
python -m unittest discover -s tests
```

The motion runner uses the same scripted sequence on Windows and macOS. It writes
`artifacts/inspector-motion/trace.jsonl`, a chronological frame strip named
`transition.png`, logs, and `report.json`. It exercises different answer cards,
diagrams, JSON, and rapid selection reversals. The report verifies that expected
layouts appeared, fading occurred, geometry stayed finite, and motion settled.

These are forced headless CPU measurements at a virtual 60 Hz. They do not measure
native display FPS, GPU completion time, or input latency. Compare the same source
revision, build profile, renderer scale, and workload. Use `--binary` to select a
performance build and `--out` to keep each machine's evidence separately.

The app uses Rune's matched geometry for the request marker, `RollingText` for
counters, an opacity loop for loading placeholders, and a short fade/lift for
detail replacement. The header and tabs stay fixed. Removed detail surfaces leave
immediately so old and new payloads cannot overlap or remain interactive. Rune's
system reduced-motion handling removes travel and stops the loading loop; an
in-place fade can remain. Idle completed captures do not run animation loops.

The Rust tests check interrupted transitions, selection consistency, tour controls,
reduced motion, and loading cleanup. The Python integration test sends all catalog
cases through the actual local relay and checks response bytes and captured data.
Native clipboard, file-picker, keyboard-delivery, and screen-reader checks remain
separate from this renderer/runtime evidence.
