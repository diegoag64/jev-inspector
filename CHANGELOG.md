# Changelog

Notable user-visible changes. This project has not published a versioned release yet; `main` is the only supported version.

## Unreleased

### Added

- **First public release.** Native inspector for TypeSafe API traffic with a local capture relay (`jev-bridge`), live traffic filters and search, formatted JSON and property trees, answer diagrams, exchange comparison, cost estimates, and session export.
- **Synthetic demo.** `dev.py demo` runs a mock API, the relay, and the inspector with no API key.

### Fixed

- **Demo startup.** `dev.py demo` now stops with a clear message on Python older than 3.11.4, finds cargo in `~/.cargo/bin` when the shell hasn't added it to PATH, and reports a missing cargo or failed build without a traceback.
