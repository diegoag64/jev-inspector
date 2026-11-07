# Contributing

Keep changes small enough to explain and verify locally. This project is a Rust
desktop app and relay, with Python tools for demos and QA. There are no CI or
deployment workflows.

## Build

Install the prerequisites in [README.md](README.md), then run from the repository
root:

```sh
python tools/prepare_rune.py
cargo build --workspace --locked
```

Use `python3` on macOS or `.venv\Scripts\python.exe` on Windows if needed. The
Python demo uses the standard library. Screenshot comparisons also need Pillow:

```sh
python -m venv .venv
```

Install with `.venv\Scripts\python.exe -m pip install -r requirements.txt` on
Windows or `.venv/bin/python -m pip install -r requirements.txt` on macOS. Use
that interpreter for Python QA commands below.

Run `python dev.py demo` for traffic through the relay, or
`cargo run -p jev-inspector -- --showcase` for offline examples.

## Make a change

Keep UI code in `jev-inspector`, reusable graph behavior in `rune-diagram`, relay
behavior in `jev-bridge`, and shared wire types in `jev-protocol`. Add synthetic
examples to `examples/demo-requests.json`; do not commit real request bodies,
tokens, local connection files, or generated captures.

Do not edit `.upstream` as the source of a permanent fix. Rune changes belong in
tracked patches and the preparation script. See [the patch guide](docs/rune-focus-patch.md).
Keep the unpatched gallery checkout separate so its comparisons remain useful.

For UI work, check both themes and the minimum window size. Exercise empty,
loading, error, and saved-session states when the change affects them. Check
keyboard focus and reduced motion as well as the normal pointer path. Preserve
manual pan and zoom when changing graph framing.

## Verify

For Rust changes:

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

For Python changes:

```sh
python -m unittest discover -s tests -p 'test_*.py'
```

For inspector layout or interaction changes, rebuild and run the relevant
fixtures with `python tools/inspector_qa.py`. See the
[workspace guide](docs/inspector-workspace.md) and
[motion checks](docs/inspector-examples.md) for options. Inspect the resulting
images; a passing state assertion does not establish that a layout is readable.
For shader changes, also run `python tools/inspector_effects.py` to check rendered
motion and settling in both themes. Use the [component QA guide](docs/component-qa.md) for upstream gallery work.

Add regression coverage for changed behavior and reproduced bugs. Do not add
tests that only repeat a constant or mirror an implementation. Record what you
actually ran, and distinguish headless checks from native OS checks. Leave
unavailable platforms and integrations marked untested.

Documentation-only changes need working links and accurate commands, not a full
Rust rebuild. Update the relevant guide whenever a control, flag, or default
changes.

## Commit convention

Use this format for new commits. Existing history does not need rewriting.

```text
type(scope): describe the change
```

Use a short imperative summary. Choose the type by the purpose of the change:

| Type | Use |
| --- | --- |
| `feat` | New user-visible behavior |
| `fix` | Correct existing behavior |
| `docs` | Documentation |
| `refactor` | Code structure without a behavior change |
| `test` | Test coverage or fixtures |
| `perf` | Measured performance improvement |
| `chore` | Dependencies, tooling, or repository maintenance |

Scopes include `inspector`, `bridge`, `protocol`, `diagram`, `qa`, and `repo`.
Omit the scope when a change spans the project and no single scope fits.

```text
fix(diagram): fit the graph on first open
feat(inspector): add an ASCII loading effect
docs(repo): explain setup and local checks
```

Keep each commit focused and runnable. Add a body when the reason or tradeoff is
not clear from the diff. Mention a breaking change explicitly and explain how
users migrate. Avoid claims such as "fully tested" or "production ready" without
evidence. Do not rewrite shared history to improve old commit wording.

## Describe the result

For a PR or handoff, explain the problem, the resulting behavior, and the checks
you ran. Include before/after images for visible changes when they help a reviewer.
List known limitations and untested environments. Keep generated QA reports out
of Git unless a small, reviewed fixture is needed to reproduce a bug.

Pushing publishes your commits; it does not deploy the app. Do not add Actions,
release automation, or automatic screenshot approval as part of routine changes.
