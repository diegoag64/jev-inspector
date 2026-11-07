# Review UI states and transitions

Use this checklist for inspector UI reviews. A screenshot of a completed request does not cover the states that precede it or the ways loading can fail.

## Why the loading view escaped the earlier reviews

The earlier review packet contained 24 screenshots. None showed an uncached exchange loading. The default demo and saved fixtures supplied captured bodies immediately, so normal tab screenshots bypassed the loading branch in `detail_view.rs`.

The rubric listed empty, error, disconnected, paused, and saved-session states, but did not name loading, in-flight bodies, or recovery transitions. A runtime test checked copying a request ID while detail was absent. That test checked the copy action, not the loading layout.

Both reviewers inspected source, so lack of source access was not the problem. The evidence packet omitted the visual state, and the review process did not require an independent inventory of UI branches. That likely contributed to the miss. The resulting scores described the inspected fixtures too broadly. Neither review established complete native Windows usability.

## Inventory before scoring

For each visible branch, record its trigger, visual fixture, behavior test, and any missing evidence. Reviewers must inspect the source independently and add states the supplied packet omits.

| Area | States and transitions to inspect |
| --- | --- |
| Connection | First launch, connecting, connected, lost connection with cached detail, lost connection without cached detail, reconnection |
| Detail loading | All five tabs before capture arrives; short and prolonged waits; success, failure, retry; selecting another request before an older fetch completes |
| Captured bodies | Request still sending, response in flight, completed empty body, valid JSON, missing capture, invalid JSON, compressed or truncated capture, summary-only session |
| Traffic | No traffic, no search matches, invalid query, cleared view, restored view, pause/resume, Follow, sorting, stars, retention eviction and relay restart |
| Sessions | Empty saved session, summary-only exchange, mixed capture availability, long filename, import failure, open cancellation, save cancellation, export progress, failed export |
| JSON | Code and tree, long scalar values, deep nesting, bounded preview, no find matches, navigation between matches, copy feedback, changing selection while Find is open |
| Comparison | No baseline, same exchange, changed request only, changed response only, missing bodies, collapsed unchanged sections, split/unified, expanded/minimum size |
| Diagrams | Missing answers, non-question endpoints, errors, long labels, multiple questions, fit/actual size, keyboard pan and zoom |
| Overflow | Every scrollable pane at rest, wheel/trackpad, thumb dragging, keyboard scrolling, large content, compact rows and both window sizes |
| Dialogs | Open/close, keyboard containment and return focus, overflow cues, disabled/busy controls |
| Motion | Reduced motion, loading-to-content replacement, rapid selection changes, stale response arrival, no continuous animation after loading ends |

Check light and dark themes, the 1100x700 minimum, the usual 1440x900 size, and a renderer scale of 1.5. A renderer scale test does not establish monitor switching or OS display scaling behavior.

## Visual evidence

Run `python tools/inspector_qa.py` after building. The runner includes loading views for all tabs, failed detail loading, disconnected uncached detail, pending and empty responses, first use, empty saved sessions, no matches, and unset comparison alongside the established workflows.

Inspect each PNG and its matching tree. A successful render only proves that the renderer produced an image. State assertions prove which branch was reached. Neither alone proves that the result looks good.

For transitions, inspect the state before, during, and after the action. Runtime tests cover request selection races, retry signals, loading replacement, placeholder accessibility, and distinct missing-body states. They do not establish OS input delivery or spoken announcements.

Run `python tools/inspector_motion.py` for a repeatable mixed-layout trace and frame
strip. Include the shared example catalog in the branch inventory: a large request
is not a large response, a language code is not evidence of Unicode glyph rendering,
and collapsed tree nodes do not establish expanded-tree usability. Exercise tour
start/pause, manual interruption, filters and dialogs; verify the final idle state.
Measure intermediate frames with Rune's delta-time API rather than advancing each
test frame by an entire second. Keep native display fluidity distinct from forced
headless CPU timing.

## Score with explicit limits

Keep the ten review categories separate. List defects before optional enhancements. Do not let an overall average conceal a weak category or award a target score to end iteration.

For each claim, distinguish rendered evidence, runtime tests, source inspection, and native OS verification. Report unperformed checks as unverified. Native clipboard paste, actual file-picker interaction, keyboard delivery, and screen-reader announcements still require a working desktop-control connection. macOS requires its own run.

Previously overlooked areas that need explicit attention include resting scrollbars outside the traffic list, horizontal JSON overflow, first-use and comparison copy, loading without a cached exchange, saved sessions without bodies, and reduced-motion behavior. An inventory entry is a review obligation, not a claim that the feature has passed.

## Findings from the complete example reviews

The complete reviews found defects that the earlier sample reviews missed:
showcase export used legacy demo captures, Open could leave the tour running while
a picker was pending, and compact diagrams started below a readable scale.
Static example screenshots did not exercise export fidelity or pending dialogs.

Regression checks now compare all 14 exported captures with their originals and
repeat the check for filtered exports. Runtime tests hold both pickers pending,
cancel them, and cover failed import. Diagram tests check initial answer framing,
Fit after layout and resizing, and preservation of manual camera changes.
The follow-up review found that checking only the answer card's top edge missed
clipped confidence and selected-value rows. Check its bottom edge too. Ordinary
cards must fit completely at minimum size. Taller cards must expose overflow and
provide working navigation to both ends; inspect both resulting views.

Keyboard inspection must verify visible focus, not just a focused flag or a
reachable key. Walk Tab and Shift+Tab after manual panning, then browse every card
and its controls with arrows. Keep the focused bounds inside the canvas and
verify that navigating to a disclosure does not activate it. The card selection
group now reveals the active card without changing zoom.

Also start the tour after panning away, then Shift+Tab from its cleared-focus
state. Element-level handlers cannot see that first key. Inspect overflow inside
each card independently of the outer frame: scalar-only properties must expose a
resting thumb and accept Page Up/Down and Home/End through their focused viewport.
Test Find by clicking its icon and typing immediately, as well as by shortcut.
Check accessible names on the tree widget itself, not only its named rows.

Test mixed pointer/keyboard scrolling, not just each input separately: drag the
properties thumb, then Page Up, Page Down, Home, and End. Traffic must not move.
Measure half- and full-thumb travel at diagram zoom below and above 100%,
independently of renderer scale. Expand a nested object that overflows, then page
while its disclosure owns focus; the resulting focus bounds must remain visible.
Review the tracked Rune focus patch and its preparation path alongside these tests.

Measure selected-row foreground/background pairs in both themes and row
densities. The contrast regression reads actual Runtime element colors for
metadata, timing, status and selection surfaces, including pending and error
rows. It requires at least 4.5:1 for each text element; it is not a full WCAG audit.

Find and Copy tests navigate between requests during unfinished transitions, with
and without reduced motion. They check that the current search field owns focus,
receives typing, and copies exactly the current JSON. This caught a focus loss
caused by blurring the field while asking it to regain focus. Clipboard payload
assertions do not establish OS clipboard delivery.

The long-JSON fixture now includes a genuinely long scalar. Inspect its horizontal
thumb as well as vertical overflow. Runtime tests drag both axes and verify that
one preserves the other's offset; short content must hide both thumbs. Overview,
JSON code, tree, comparison, and Traffic need separate resting-overflow checks.

The Windows desktop connection was retried on 2026-11-06. It still reports
"Computer Use native pipe is unavailable" (OS error 2). Clipboard paste, picker UI,
OS keyboard/focus and screen-reader announcements remain not tested through the
native desktop. Do not replace these results with runtime-test passes.
