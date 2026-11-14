# Repository instructions

## Always apply

- Work on a branch and open a pull request against `main`. Don't commit or push directly to `main` unless the maintainer explicitly asks. Merging requires explicit authorization.
- Never commit real request bodies, API keys, `.jev/` connection files, or generated captures. Use synthetic data from `examples/demo-requests.json` and `tests/fixtures/`.
- Don't edit `.upstream/` as the source of a fix. Rune changes belong in `patches/` and `tools/prepare_rune.py`. See [docs/rune-focus-patch.md](docs/rune-focus-patch.md).
- Diagrams show captured inputs and returned values only. Don't invent model reasoning or routing policy.

## Read by task

- Setup, build, checks, and source ownership: [CONTRIBUTING.md](CONTRIBUTING.md).
- Relay behavior, ports, and capture limits: [docs/bridge-quickstart.md](docs/bridge-quickstart.md) and [docs/bridge-v1.md](docs/bridge-v1.md).
- Inspector controls, shortcuts, and headless screenshots: [docs/inspector-workspace.md](docs/inspector-workspace.md) and [docs/inspector-examples.md](docs/inspector-examples.md).
- UI reviews: inventory the affected states with [docs/ui-review-coverage.md](docs/ui-review-coverage.md) before judging a layout.
- Diagram components: [docs/diagrams.md](docs/diagrams.md).

## Preserve boundaries

Keep UI code in `crates/jev-inspector`, reusable graph behavior in `crates/rune-diagram`, relay behavior in `crates/jev-bridge`, and shared wire types in `crates/jev-protocol`. `jev-bridge` must not depend on Rune.

## Verify changes

Run `python3 tools/prepare_rune.py` once before building. Select checks for the changed behavior:

- Rust changes: `cargo fmt --all --check`, `cargo test --workspace --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Python changes: `python3 -m unittest discover -s tests -p 'test_*.py'`.
- Inspector layout or interaction changes: rebuild, run the relevant cases with `python3 tools/inspector_qa.py --case <name>`, and look at the PNGs. Check both themes and the 1100x700 minimum size.
- Documentation-only changes: check local links, command names, and `git diff --check`.

When launching the app for the user to test (`python3 dev.py demo`), run it outside any sandbox so the native window is visible. Report the checks run, failures, and untested behavior, and distinguish headless renders from native checks.

## Maintain documentation

Keep the root README focused on what the project is, startup, and documentation links. Put details in `docs/`. Update the relevant guide whenever a control, flag, or default changes. Regenerate README screenshots in `docs/images/` from synthetic data (`--showcase` or fixtures) when the UI changes noticeably.

Add notable user-visible changes to `CHANGELOG.md` under Unreleased, grouped under Added, Changed, or Fixed.

## Pull requests

Follow [CONTRIBUTING.md](CONTRIBUTING.md#pull-requests) for titles (`<type>(<scope>): <summary>`) and fill in [the template](.github/pull_request_template.md). Attach before-and-after captures for visible changes as GitHub attachments, not committed files; check `gh pr edit --help` for `--attach` support, or list local paths and mark attachments pending. End the description with the model and tool that did the work.
