# Contributing

Thanks for helping. Keep changes small enough to explain and verify locally. This
project is a Rust desktop app and relay, with Python tools for demos and QA.

Open an issue before starting a large change, so we can agree on the direction
first. Report security problems privately; see [SECURITY.md](SECURITY.md). By
taking part, you agree to follow the [code of conduct](CODE_OF_CONDUCT.md).

## Workflow

1. Fork the repository, or create a branch from current `main` if you have write
   access. Don't push directly to `main`.
2. Make one focused change and run the [checks](#verify) it needs.
3. Add an entry under **Unreleased** in [CHANGELOG.md](CHANGELOG.md) for
   user-visible changes.
4. Open a pull request against `main` using the [template](.github/pull_request_template.md).

## Build

Install the [prerequisites](README.md#try-it-locally), then run from the repository
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
Use the [component QA guide](docs/component-qa.md) for upstream gallery work.

Add regression coverage for changed behavior and reproduced bugs. Do not add
tests that only repeat a constant or mirror an implementation. Record what you
actually ran, and distinguish headless checks from native OS checks. Leave
unavailable platforms and integrations marked untested.

Documentation-only changes need working links and accurate commands, not a full
Rust rebuild. Update the relevant guide whenever a control, flag, or default
changes.

## Commit convention

Use the same format as PR titles for commit subjects:

```text
type(scope): describe the change
```

Keep each commit focused and runnable. Add a body when the reason or tradeoff is
not clear from the diff. Do not rewrite shared history to improve old commit
wording.

## Pull requests

Use `<type>(<scope>): <summary>` for every PR title, within 72 characters:

```text
fix(diagram): fit the graph on first open
feat(inspector): add saved-session search
docs(repo): explain setup and local checks
```

Use a lowercase type and choose it by the purpose of the change:

| Type | Use |
| --- | --- |
| `feat` | New user-visible behavior |
| `fix` | Correct existing behavior |
| `docs` | Documentation |
| `refactor` | Code structure without a behavior change |
| `test` | Test coverage or fixtures |
| `perf` | Measured performance improvement |
| `build` | Build scripts, Rune preparation, or dependencies |
| `chore` | Repository maintenance |

Scopes include `inspector`, `bridge`, `protocol`, `diagram`, `qa`, and `repo`.
Choose the primary area when a change spans several. Write the summary as an
imperative phrase with no trailing period. For a breaking change, put `!` before
the colon and explain the migration in the description.

Fill in the [PR template](.github/pull_request_template.md):

- **Summary:** the problem and the resulting behavior. Link related issues.
- **Screenshots:** before and after images for visible changes, at the same window
  size, scale, and theme. Attach them to the PR rather than committing them.
  Intentional documentation images belong under `docs/images/`.
- **Validation:** the commands and interactions you checked, with results. Name the
  OS and whether UI checks were native or headless. State what you didn't test.

There is no hosted CI, so the Validation section is the verification record. Avoid
claims such as "fully tested" without evidence. Keep generated QA reports out of
Git unless a small, reviewed fixture is needed to reproduce a bug. Update the title
and description to match the final scope before requesting review.

If an AI agent did some or all of the work, name the model and tool at the end of
the description. Agent instructions live in [AGENTS.md](AGENTS.md).
