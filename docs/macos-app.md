# Run Jev Inspector as a Mac app

Package the inspector as `Jev Inspector.app`, an app you can double-click. The app runs the relay itself, so you don't need a terminal, Rust, or Python to use it after it's built.

## Build the app

You need the [build prerequisites](../README.md#try-it-locally) and Pillow from `requirements.txt` (used for the app icon).

```sh
python3 tools/bundle_macos.py             # dist/Jev Inspector.app
python3 tools/bundle_macos.py --install   # also copy it to /Applications
python3 tools/bundle_macos.py --dmg       # also build dist/Jev-Inspector.dmg
```

The script prepares the pinned Rune sources, makes a `release` build of `jev-inspector`, adds an Info.plist and a generated icon, and signs the bundle ad hoc. Add `--open` to launch the app when the build finishes.

An ad hoc signature works only on the Mac that built the app. Other Macs need a Developer ID signature and notarization before Gatekeeper opens the app; the script doesn't do either.

## First launch

The first time it opens, the app walks you through setup:

1. **Welcome.** What the inspector shows.
2. **Port.** The local port your backend sends requests to. The default is `8787`. The field checks whether the port is free as you type. If another app already uses it, the app suggests the next free port. Ports must be from 1024 to 65535.
3. **Capture.** Full capture (JSON bodies, answers, and diagrams) or metadata only. This is the same as running `jev-bridge` with or without `--capture-bodies`.
4. **Connect.** The relay starts. Copy the `export TYPESAFE_BASE_URL=…` line into your backend's terminal and restart the backend. The step updates when the first request arrives.

## How the port works

- The relay listens on `127.0.0.1` only, on the port you chose. Your backend sends requests there, and the relay forwards each one unchanged to `https://api.typesafe.ai`.
- The inspector reads captures through a second loopback port that the system assigns. You never configure it.
- The app saves your choice in `~/Library/Application Support/dev.jev.inspector/prefs.json` and reuses it on every launch.
- The relay runs while the app is open. Quitting the app stops the relay and clears its history.
- If the saved port is busy at launch, the app opens Relay settings with an error so you can pick another.

To change the port or capture mode later, click **Port 8787** in the toolbar, or **Relay settings…** in Connection setup. If the new port fails to start, the previous relay keeps running.

## Stop and start the relay

Click the stop button next to **Port 8787** in the toolbar, or **Stop relay** in Connection setup. The port is released right away, and your backend's requests to it fail until you start the relay again. The toolbar badge changes to **Relay stopped**, and a banner offers **Start relay**.

Requests you already captured stay in the list. Exchanges you opened before stopping keep their details; others may show as unavailable, because the relay's history clears when it stops.

**Start relay** (the banner, the toolbar play button, or Connection setup) restarts it on the saved port. If another app took the port while the relay was stopped, Relay settings opens so you can pick another. The app always starts the relay when it launches, even if you stopped it last time.

## Developer builds

`cargo run -p jev-inspector` still connects to a separately started `jev-bridge` through its connection file, as described in the [bridge quickstart](bridge-quickstart.md). To try app mode from a terminal, run:

```sh
cargo run -p jev-inspector -- --app
```

Two flags preview onboarding without starting a relay or saving preferences:

```sh
cargo run -p jev-inspector -- --onboarding            # from the welcome step
cargo run -p jev-inspector -- --onboarding-step 1     # 0 welcome, 1 port, 2 capture, 3 connect
```

The headless QA cases `onboarding-welcome`, `onboarding-port`, `onboarding-capture`, `onboarding-connect` (each with a `-light` variant), and `onboarding-received` render each step:

```sh
python3 tools/inspector_qa.py --case onboarding-port
```
