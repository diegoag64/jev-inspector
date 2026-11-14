# Review UI states and transitions

Use this checklist for inspector UI reviews. A screenshot of a completed request does not cover the states that precede it or the ways loading can fail.

## Inventory UI states

For each visible branch, record its trigger, visual fixture, behavior test, and any missing evidence. Reviewers must inspect the source independently and add states the supplied screenshots omit.

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

Repeat thumb and track interactions on an inactive card after Fit, both with no
explicit card selection and with another card selected. Test a click and paging
key queued before a frame, not only settled input sequences. Activation must
preserve the camera and Traffic offset, and switching cards must remove the old
properties from keyboard traversal.

Measure rendered keyboard focus outlines independently of selected-card borders
and text contrast. Check headings and properties in both themes, at Fit and
working zoom, and at renderer scales 1 and 1.5. Preserve keyboard-only visibility.
Run `python tools/inspector_focus_contrast.py --packet <main-packet> --packet
<dpi-packet> --out <report.json>` after rendering the eight focus cases at both
scales. It samples actual stroke and neighboring background pixels and requires
at least 3:1 contrast; it does not establish native or complete accessibility.
Also batch separate host updates before a frame: a later blur must cancel an
earlier pending focus request, while a subsequent focus request must still work.

Each review round must revisit complete user workflows, rather than only the last
reported defect. Independently identify missing states, unnecessary steps,
unclear terminology, hierarchy problems, and recovery paths outside the supplied
packet. Rank improvements by user impact and distinguish product defects from
unverified native behavior. Reviewers with different scopes should still report
coverage limits and independent scores; an aggregate score cannot hide a weak area.

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

## Broad workflow boundaries

The broader reviews found cases that repeated screenshots of the same fixtures
did not exercise. Reviewers must vary data size and combine states, then trace
the result back to the captured data. In particular, check:

- A search term beyond byte 16,000, including Unicode; absent terms and match navigation.
- Equal comparison prefixes with different tails beyond the byte and line limits.
- Typing incomplete filters while a non-default exchange is selected, then correcting or clearing.
- Saved pending and partially captured responses, plus live pending while disconnected.
- Six, twelve, and twenty-four long question names at minimum window size and both DPI scales.
- Thirteen and forty probability categories with the returned choice beyond the leading twelve.
- Summary-only metadata with Traffic hidden, relay restart, and selected-exchange eviction.
- First-use connection guidance from both the empty workspace and Help.

Keep native OS checks separate from Runtime assertions. A passing headless
clipboard effect or picker simulation does not verify the operating system.

## Verify combinations and visible results

The workflow reviews found gaps in tests that checked a search count or one
response tab without checking the surrounding workflow. A positive match count
does not prove the matching glyphs are visible. Correct saved-response wording
does not establish correct Overview wording.

- Find a term after a long CJK string, emoji, combining marks, and mixed text.
  Check the shaped match bounds against the viewport in both themes and scales.
- Keep Find open while switching short and long captures, returning from another
  tab, and returning from Tree to Code. Scroll manually after the initial reveal
  to check that layout updates do not keep moving the document.
- Inspect saved pending, saved partial, and disconnected pending exchanges in
  every applicable tab. Preserve recorded headers when total timing is absent.
- Use a custom connection path containing spaces and an apostrophe, plus a custom
  relay port. Compare the copied relay, inspector, and backend commands.
- Give multiple questions identical long prefixes and different suffixes. Check
  the closed selector, open menu, diagram heading, keyboard selection, and full-name help.
- Place the returned choice just inside and outside each probability disclosure
  boundary. Compare the button's count with the number of rows actually added.
  Revisit the exchange to check that its disclosure state survives navigation.
- Compare bodies whose only difference is outside the preview. Open the current
  body and copy the complete baseline after its exchange leaves retained history.

When layout changes make an old overflow fixture fit, exercise overflow at a
larger zoom. Keep the ordinary-size fit check too. Derive pointer coordinates
from the newly rendered scrollbar bounds before rerunning track-click checks.

## Audit corresponding views together

The recovery reviews found repeated concepts implemented in separate views.
Overview's probability disclosure was correct while Diagram's count was not.
Diagram kept full question names while Overview discarded their suffixes.
Timing described saved and disconnected requests correctly while headers,
filters, and absent-answer messages still implied live progress.

For each finding, inventory every place that displays or acts on the same data:

- Check answer identity in Overview, the question menu, diagram headings, hover
  help, and accessible names. Use identical prefixes with distinct suffixes.
- Check pending state in all tabs, headers, aggregate counts, filters, and saved
  partial results. Confirm that useful captured results remain scrollable.
- Count actual additions in both probability disclosures, including an already
  visible returned category and singular button wording.
- Exercise global shortcuts while Help, Export, and the question menu are open.
  Escape must close the current modal and restore ordinary navigation.
- Navigate from a selection excluded by nonempty filters in both window layouts.
- Save with a missing connection file, a known outage, and a connection lost
  during refresh. Reopen and resave the fallback snapshot. Confirm provenance,
  stars, missing bodies, captured values, and protection against relay restarts.
- Inspect setup commands with custom upstream, listener, history, and connection
  path together. Connected setup must lead with the remaining routing task.
- Cross match 1,000 in both directions and wrap at the actual final match. Include
  Unicode and dense repeated patterns that overlap preview boundaries.
- Check measured probability headlines and confidence against exact zero, tiny
  positive, ordinary fractional, near-one, and exactly one values. Axis endpoint
  labels are reference values; they must not be confused with measured values.
- Compare an incomplete baseline with a complete current capture, then reverse
  the roles. Check both incomplete, missing JSON, expansion, and returning to
  Compare. Each side must retain its capture context even when captured JSON is equal.
- Reopen a saved summary-only request with an existing live connection error,
  then reconnect. Historical details must never promise automatic completion.

## Keep scripted navigation tied to its purpose

Target scrollbar keys instead of fixed screen coordinates. Use End when a case
checks the final property; Page Down checks one viewport of movement. Before
checking both tall-card reveal directions, zoom until the card exceeds the
viewport. Keep the assertions for the resulting focus and visible content.

Rune treats a control inside a focused scroll region as a separate Tab stop.
Use Tab to enter its nested disclosures, and include every toolbar stop when
testing keyboard entry into the diagram.

The focus-contrast check counts all sampled pixels with at least 3:1 contrast
against the adjacent background. At least half the sample must qualify.
Antialiased pixels can have different colors without reducing stroke coverage.
