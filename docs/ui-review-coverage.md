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
