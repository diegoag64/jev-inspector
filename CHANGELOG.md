# Changelog

Notable user-visible changes. This project has not published a versioned release yet; `main` is the only supported version.

## Unreleased

### Added

- **First public release.** Native inspector for TypeSafe API traffic with a local capture relay (`jev-bridge`), live traffic filters and search, formatted JSON and property trees, answer diagrams, exchange comparison, cost estimates, and session export.
- **Synthetic demo.** `dev.py demo` runs a mock API, the relay, and the inspector with no API key.
- **Mac app.** `tools/bundle_macos.py` builds `Jev Inspector.app`, which runs its own relay so it opens with a double-click. First launch walks through choosing the relay port, which checks whether the port is free and suggests another if it isn't, then the capture mode and routing your backend. Change either later from **Port** in the toolbar, and stop or restart the relay with the button beside it. See [docs/macos-app.md](docs/macos-app.md).

### Changed

- Update Rune to upstream revision `d77610b5`, retaining the inspector focus, Select label, and table row patches.
- In diagram cards, use Tab to enter nested disclosure controls from a focused properties region. Arrow keys continue to navigate the enclosing card group.
- **Native look on macOS.** The inspector uses the system font (SF Pro, with SF Mono for code) and falls back to Inter elsewhere. Panes, cards and tiles use larger continuous corners and Apple's grouped light and dark surfaces. Detail tabs, Traffic filters and Help sections are segmented controls. Labels and badges use sentence case, numbers use tabular figures, and scrollbar thumbs are slimmer.

### Fixed

- **Demo startup.** `dev.py demo` now stops with a clear message on Python older than 3.11.4, finds cargo in `~/.cargo/bin` when the shell hasn't added it to PATH, and reports a missing cargo or failed build without a traceback.
