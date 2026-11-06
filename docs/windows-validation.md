# Initial Windows validation

Rune revision: `781e2b6d87cd5f32d6f4365956fad4baaf24ee50`.

The setup uses upstream Rune without patches. The optimized `perf` build succeeded on this Windows x64 machine with Rust 1.96.0. One upstream dead-code warning appeared during compilation.

## Verified behavior

- The 12 selected upstream component test targets passed 61 tests. Two upstream stress tests were ignored.
- All 168 captures passed: 28 cases in two themes at scales 1, 1.25, and 2. No element-tree warnings were reported.
- All five headless benchmark workloads completed one warmup and three measured runs.
- Native timing completed one warmup and three measured runs of 240 frames. The trace reported a 960×720 point viewport at 1× scale and 240 Hz.
- Comparing the capture report with itself produced zero changed pixels across all 168 images. Two independently generated button-state captures also matched exactly.
- A real native window opened using DirectX 12 on NVIDIA GeForce RTX 4080 SUPER.
- Clicking Open dialog displayed the modal and moved focus to its Close button.
- Tab moved focus to Done inside the modal.
- Escape dismissed the modal and returned focus to Open dialog.
- Windows UI Automation exposed the component hierarchy and controls.
- Sample button-state and Unicode-input PNGs were visually inspected. Accented Latin and Japanese text rendered in the input fixture.

The native window was closed after the check. These observations do not establish full screen-reader support or native IME behavior.

## Adapter selection

Without overrides, headless captures selected Intel UHD Graphics 770. The native window selected the NVIDIA GPU driving the primary display. Both used DirectX 12.

This distinction matters when reproducing a slowdown. Use `WGPU_ADAPTER_NAME` to select a GPU and retain the actual GPU log. Do not compare an offscreen Intel measurement to a native NVIDIA measurement as if only the window mode changed.

GPU initialization varied during the capture sweep, including processes reporting roughly 11 seconds after earlier subsecond initialization. Those captures ran while development and native checks were in progress. Treat this as an observation to reproduce in an isolated run, not an established Rune regression. The logs preserve the timing summaries.

The subsequent headless benchmark, run without concurrent captures or builds, also recorded roughly 11-second Intel GPU initialization. Steady frame time is measured separately. The cause has not been isolated to Rune, its GPU dependencies, the driver, or this execution environment.

Generated evidence is under `artifacts/windows-matrix`, `artifacts/windows-behavior-verified`, `artifacts/windows-benchmark`, and `artifacts/windows-native`. These folders are local and ignored by Git. The initial measurements predate a final report-only change that checks refresh-rate compatibility; their recorded runner hashes remain unchanged. Generate fresh runs for a strict comparison with future Mac results.

## Limits

No macOS hardware was used in this session. macOS build and runtime results remain unverified locally; the repository includes commands to run checks on that machine.

Native OS display scaling, multiple monitors, IME composition, clipboard integration, Narrator, long idle periods, and memory growth require the manual checklist. Headless DPI variants and accessibility-tree assertions do not replace those checks.

The initial development captures are diagnostic evidence, not approved regression baselines. Create a fresh run after reviewing the fixtures and choosing your comparison settings.
