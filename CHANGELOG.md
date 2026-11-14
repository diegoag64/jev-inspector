# Changelog

Notable user-visible changes. This project has not published a versioned release yet; `main` is the only supported version.

## Unreleased

### Added

- **First public release.** Native inspector for TypeSafe API traffic with a local capture relay (`jev-bridge`), live traffic filters and search, formatted JSON and property trees, answer diagrams, exchange comparison, cost estimates, and session export.
- **Synthetic demo.** `dev.py demo` runs a mock API, the relay, and the inspector with no API key.

### Changed

- Update Rune to upstream revision `d77610b5`, retaining the inspector focus, Select label, and table row patches.

- **Native look on macOS.** The inspector uses the system font (SF Pro, with SF Mono for code) and falls back to Inter elsewhere. Panes, cards and tiles use larger continuous corners and Apple's grouped light and dark surfaces. Detail tabs, Traffic filters and Help sections are segmented controls. Labels and badges use sentence case, numbers use tabular figures, and scrollbar thumbs are slimmer.

### Fixed

- Count all sufficiently contrasting antialiased pixels when checking focus-ring coverage.
- Update diagram QA fixtures for Rune's nested-control Tab navigation and current text layout. Scrollbar checks target controls by key instead of fixed coordinates.

- **Demo startup.** `dev.py demo` now stops with a clear message on Python older than 3.11.4, finds cargo in `~/.cargo/bin` when the shell hasn't added it to PATH, and reports a missing cargo or failed build without a traceback.
