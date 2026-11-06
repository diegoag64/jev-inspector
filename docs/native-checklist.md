# Check native component behavior

Copy this file into a run directory and fill it in while using `qa.py gallery`. Mark each item pass, fail, or not tested. A synthetic script does not establish that the equivalent OS interaction works.

## Record the environment

- Date and Rune revision:
- Windows or macOS version:
- CPU, GPU, and GPU driver:
- Monitor resolution and refresh rate:
- Display scale and number of monitors:
- Power mode and plugged-in status:
- Theme and reduced-motion preference:
- Keyboard layout and input method:
- Report folder and reproduction command:

## Check input and focus

Open `motion-focus-modal`.

- Use Tab and Shift+Tab through the controls. Check focus indicators and tab order.
- Toggle Sync and Remember by pointer and Space. Check one change per activation.
- Move between First and Second with arrow keys.
- Open the dialog with the pointer. Check initial focus, Tab wrapping, and background input blocking.
- Press Escape. Check that the dialog closes and focus returns to its opener.
- Open and close it repeatedly while resizing the window.
- Switch to another app and back. Check that focus and pressed states recover.

Open `ui-switch--states` and `ui-checkbox--states`.

- Click disabled controls and try to focus them with Tab. Check that they do not activate.
- Press an enabled control, drag outside it, and release. Check that the release cancels activation.

## Check editing

Open `ui-text-input--playground` and `ui-textarea--playground`.

- Type ASCII, accents, emoji, and text from an IME. Check composition before committing.
- Select with pointer, Shift+arrows, Home, End, and platform word-navigation shortcuts.
- Copy and paste multiline text using Ctrl on Windows and Command on macOS.
- Test undo and redo, selection replacement, horizontal movement, and multiline wrapping.
- Resize during a selection. Check the caret and selection remain aligned with glyphs.

Open `rune-combobox--searchable` and `ui-select--grouped`.

- Filter, move with arrows, choose with Enter, and cancel with Escape.
- Scroll with a physical wheel and a trackpad. Check direction, momentum, and nested scrolling.
- Open the popup near each window edge. Check clipping and repositioning.

## Check display and accessibility

- Compare light and dark themes at 100%, 125%, 150%, and 200% OS display scaling.
- Move the window between monitors with different scaling and refresh rates.
- Check text baselines, borders, icons, glyph fallback, clipping, and focus-ring edges.
- Minimize, restore, maximize, and resize. Check for flicker, blank frames, or stuck pointer states.
- Test reduced motion separately from regular motion.
- Use Narrator on Windows or VoiceOver on macOS. Check names, roles, values, focus order, and disabled state announcements.
- Leave the window idle, then animate and resize it for several minutes. Observe resource use externally; this runner does not measure memory leaks.

## Record a failure

1. Save the exact launch command and the shortest interaction sequence.
2. Record expected and actual behavior.
3. Attach the matching screenshot, tree, log, and run metadata if available.
4. Repeat on the same machine and then on the other OS with the same revision.
5. State whether the failure occurs headlessly, natively, or both.

Do not infer an OS bug from a screenshot difference alone. Rune's default font smoothing, GPU backend, and adapter selection can differ across platforms.
