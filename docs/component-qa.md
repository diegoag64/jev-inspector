# Rune component QA

Run these commands from the repository root.


Run Rune's native components on Windows and macOS, reproduce interactions, and compare screenshots and performance. This project runs the **actual upstream gallery**, pinned to a commit, with no changes to Rune.

Start here on this Windows machine:

```powershell
.\.venv\Scripts\python.exe qa.py gallery --filter button
```

Choose a scene number. A native window opens. Close it to return to the terminal. To open the interaction fixture directly:

```powershell
.\.venv\Scripts\python.exe qa.py gallery motion-focus-modal
```

The repository contains the runner, case definitions, comparison tools, and tests. Rune's downloaded source, compiled programs, Python environment, and generated reports are ignored by Git.

The inspector uses a separate pinned source export with the tracked
[`Tx::focus_control` patch](../patches/rune-focus.patch). It lets scrolling retain
focus on a visible properties region. `tools/prepare_rune.py` applies it on Windows
and macOS without changing the upstream gallery checkout. See
[the local patch setup](rune-focus-patch.md).

See [CONTRIBUTING.md](../CONTRIBUTING.md) for local checks and commit conventions. There are no CI or deployment workflows.

## Set up a new machine

Install Git, Python 3.11 or later, and Rust through [rustup](https://rustup.rs/). The tested compiler is Rust 1.96.0. Use the same compiler version on both machines when comparing performance.

On Windows, install Visual Studio Build Tools with **Desktop development with C++** and a Windows SDK. On macOS, install Xcode Command Line Tools with `xcode-select --install`.

Windows PowerShell:

```powershell
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -r requirements.txt
.\.venv\Scripts\python.exe qa.py doctor
.\.venv\Scripts\python.exe qa.py setup
```

macOS:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
.venv/bin/python qa.py doctor
.venv/bin/python qa.py setup
```

`setup` downloads the revision in [rune.lock.json](../rune.lock.json) and builds the gallery with Rune's optimized, non-incremental `perf` profile and its upstream Cargo lockfile. The first build takes several minutes. It does not install system packages. Pillow is used only for screenshot comparisons. The other commands use Python's standard library.

The examples below use `python`. Substitute `.\.venv\Scripts\python.exe` on Windows or `.venv/bin/python` on macOS. Activation is optional.

## Browse components

```sh
python qa.py list
python qa.py list text
python qa.py gallery --filter select
python qa.py gallery ui-text-input--playground --theme light
python qa.py gallery motion-focus-modal --kit neutral
python qa.py gallery rune-combobox--searchable
```

The pinned gallery exposes 119 scenes. `list` reads the scene names from the built binary. Each window isolates one upstream scene. The gallery's default index stacks every scene without a useful viewport, so the runner asks you to select one instead.

Some upstream scenes show forced visual states or read-only examples. For editable text, use `ui-text-input--playground` or `ui-textarea--playground`. For switches, checkboxes, segments, and modal focus, use `motion-focus-modal`. Use the [native checklist](native-checklist.md) to record desktop behavior.

## Run automated checks

```sh
python -m unittest discover -s tests -v
python qa.py test
python qa.py capture --out artifacts/windows-captures
```

`test` runs 12 upstream Rust test targets covering disabled controls, input handling, selection, dialogs, focus, comboboxes, lists, switches, trees, and tables. These tests exercise runtime behavior without an OS input device. Upstream's ignored stress tests stay ignored.

The default capture suite runs 22 visual fixtures and six interaction assertions in light and dark themes at scales 1, 1.25, and 2, producing **168 captures**. It checks PNG dimensions, finite layout rectangles, and the interaction outcomes in the accessibility tree. The unkeyed disabled switch is located from its initial layout before clicking it. Reports retain all script events, PNGs, element trees, logs, warnings, and failures.

Open the generated `index.html` directly from disk. Use its filter to find a component. A passing capture is **not** an approved visual baseline: inspect the images, then keep a run you trust for comparison.

For shorter or broader runs:

```sh
python qa.py capture --themes dark --scales 1 --only toggle
python qa.py capture --all --themes light dark --scales 1
python qa.py capture --only text-input --scales 1 1.25 1.5 2
python qa.py capture --only dialog --reduced
```

`--all` captures every gallery scene and replaces the default suite. It does not include the six interaction scripts. Output folders must be new so a run cannot overwrite earlier evidence.

## Measure performance

Close other GPU-heavy apps, connect power, and keep the machine's power mode stable. Record monitor refresh rate, OS display scaling, driver version, and power mode beside your reports. Run performance measurements without concurrent captures or builds.

```sh
python qa.py bench --label windows-desktop --out artifacts/windows-bench
python qa.py native --label windows-desktop --out artifacts/windows-native
```

`bench` runs five fixed workloads: buttons at rest, Markdown text, combobox filtering, dialog motion, and Markdown resizing. It discards one process warmup, then keeps seven runs of 360 simulated frames at 60 Hz. Results include individual samples, medians, and median absolute deviation. CPU frame measurements cover Rune's update, build, encode, and submission work. They do not measure GPU completion or display latency. `process_wall_ms` is the whole process duration, not time to first visible pixels. Virtual 60 Hz does not establish 60 FPS on a display.

`native` opens the loader scene and records 240 real window frames per run, with one warmup and three measured runs. Keep each window visible and leave input idle. Each window closes automatically. Native results use wall-clock CPU submission timestamps, exclude each run's first frame, and report actual trace dimensions and scale. Submission intervals include idle time. On macOS, Rune may also provide presentation intervals. Windows has no equivalent presentation timestamp in this trace. The two report kinds cannot be mixed.

In headless results, Rune's mean excludes the first frame. Its p50, p95, and maximum include that frame. The report preserves those upstream definitions.

The buttons and text workloads force frames for otherwise static content. They measure repeated rendering work, not native idle CPU use or text reflow on every frame. The resize workload exercises Markdown reflow.

Select workloads or sample counts explicitly:

```sh
python qa.py bench --only combobox --runs 9 --scale 2
python qa.py native components-spinner--basic --runs 5 --frames 480
```

Native traces need a continuously animating scene. A static scene may not reach the requested frame count and will fail after 120 seconds. Headless workloads reject frame counts that would cut off their scripted events.

### Select the GPU

Rune can choose different adapters for headless rendering and a visible window. On the tested Windows desktop, headless rendering chose Intel UHD Graphics 770 while the native window chose NVIDIA RTX 4080 SUPER. Every run enables Rune's GPU log.

To select an adapter, set Rune's supported name filter. PowerShell:

```powershell
$env:WGPU_ADAPTER_NAME = 'NVIDIA'
python qa.py bench --label windows-nvidia --out artifacts/windows-nvidia
Remove-Item Env:WGPU_ADAPTER_NAME
```

macOS or another POSIX shell:

```sh
WGPU_ADAPTER_NAME=Apple python qa.py bench --label mac-apple --out artifacts/mac-apple
```

`WGPU_POWER_PREF=high` is another supported selection policy. `RUNE_GPU_BACKEND=wgpu` requests wgpu instead of Rune's default native Metal backend on supported Apple builds. Logs identify the adapter actually chosen; the runner records environment overrides without changing them.

## Compare Windows and macOS

Run the same commands, compiler version, case definitions, dimensions, theme, and scale on both machines. Copy the **whole run directory**, including its images and logs, into one machine's `artifacts` directory.

```sh
python qa.py compare artifacts/windows-bench artifacts/mac-bench --out artifacts/performance-comparison
python qa.py compare artifacts/windows-captures artifacts/mac-captures --out artifacts/visual-comparison
```

Open the comparison's `index.html`. Performance reports show values, dispersion, percentage changes, and paired bars. Visual reports show both screenshots and an amplified pixel difference with changed-pixel percentages. No server or online service is needed. Reports are portable folders.

Comparisons reject different Rune revisions, compiler versions, build settings, runner code, workloads, case sets, or rendering overrides. Native comparisons also check actual viewport size, scale, and reported refresh rate. For a deliberate experiment, add `--allow-mismatch`; the report shows the mismatches. Native comparisons report only timing metrics available on both platforms. Optional visual CI thresholds use `--max-changed-percent 0.1`.

Cross-machine results combine the CPU, GPU, driver, OS, power policy, and platform defaults. They do not isolate Windows overhead. Rune defaults to font smoothing on for macOS and off elsewhere, so text pixel differences can be expected. Review them before filing a bug. CPU timings are not GPU execution times; no GPU-duration or memory-leak benchmark is claimed here.

## Add cases or change Rune versions

Edit [cases.json](../cases.json). Add a scene to `smoke`, or put timestamped events and expected accessibility values in `interactions`. Assertions require a matching node and verify every match. Events normally use stable Rune keys. `target` selectors are resolved against the initial element tree, so use them only for controls present in that initial layout.

To change Rune, set a full commit SHA in `rune.lock.json`, then run `setup` and the tests. Commit that lock change with any case changes. The runner refuses dirty upstream checkouts and binaries that differ from the recorded build. It never patches Rune or automatically blesses screenshots.

Run the checks and performance comparisons on your Windows and macOS machines. This repository has no automated CI or deployment workflows.

See [the initial Windows validation](windows-validation.md) for what was exercised here and what still needs manual testing.

Upstream references: [Rune](https://github.com/anomalyco/rune), [component documentation](https://github.com/anomalyco/rune/blob/781e2b6d87cd5f32d6f4365956fad4baaf24ee50/crates/rune-kit/README.md), and [headless tooling](https://github.com/anomalyco/rune/blob/781e2b6d87cd5f32d6f4365956fad4baaf24ee50/crates/rune/README.md#agent-tooling-headless-no-window-same-renderer-fixed-virtual-clock).
