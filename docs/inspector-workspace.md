# Inspect and save requests

Start the local demo with `python dev.py demo --skip-build`, or connect the inspector
to a relay as described in [the quickstart](bridge-quickstart.md). The demo generates
one batch of synthetic requests, then stays idle. It does not contact TypeSafe.

On macOS, the native window buttons sit inside the top toolbar. Drag an empty
part of the toolbar to move the window. Double-click it to perform the titlebar
action selected in macOS settings.

The sun or moon button switches between neutral light and dark themes. On macOS
the inspector uses the system font; other platforms use Inter. Status colors
distinguish errors and warnings; JSON keeps syntax colors. Toolbar actions use
icons with hover labels and keyboard hints. In Traffic, the pause/play icon holds
or resumes the list, the labeled tracking icon toggles Follow, and the labeled
eraser clears the view. Detail tabs, Traffic filters, and Help sections are
segmented controls; the selected option is raised above its track.

Traffic starts with compact, single-line rows. Each row shows its exchange ID and
model, with the method and endpoint available on hover. Rows without a captured
model show the endpoint instead. Click **Comfortable** in the Traffic footer for
two-line rows that show the endpoint and model together; **Compact** switches back.
Live workspaces remember your density preference. The metrics remain in a slim
strip so request details have more vertical space.

## Find a request

Enter words in the Traffic search field. Words combine with AND. Quote a phrase to
keep its words together, or prefix a term with `-` to exclude it. Search checks
metadata. Use **Find** in Request or Response to search captured JSON.

| Search | Matches |
| --- | --- |
| `status:429` | HTTP 429 responses |
| `status:4xx` | HTTP 400 through 499 |
| `status:>=400` | HTTP errors |
| `duration:>200ms` | Observed elapsed time greater than 200 milliseconds |
| `duration:>=1s -status:429` | Calls of at least one second, excluding HTTP 429 |
| `model:jev method:POST` | POST requests whose model contains `jev` |
| `path:systemone` | Paths containing `systemone` |
| `id:12` | Exchange 12 in the current relay instance |
| `request:abc` | Upstream request IDs containing `abc` |

Use the **Errors**, **Success**, **In flight**, and **Starred** buttons to narrow the
list. Click a column header to sort. Click it again to reverse the order. Drag a
column edge to resize it. The selected exchange stays selected after sorting.

The header metrics describe the visible rows, except **Observed**, which is the
relay's lifetime counter. Median and P95 use visible rows that have an elapsed time.
They describe relay-observed latency, including forwarding and upstream work.

## Estimate request costs

Click **Costs** in the Traffic footer. The list shows input tokens and estimated
USD per request. Click either column heading to sort. The total covers all rows
matching the current search and filters, including rows below the scroll viewport.
Clear and Restore also change the total. Hidden and evicted requests are excluded.
Pass `--costs` when starting the inspector to open this mode directly.

Click **Pricing** to change the input and output rates in USD per million tokens.
The default is $0.042 for input and $0 for output, based on
[TypeSafe's model pricing](https://docs.typesafe.ai/models), checked November 7, 2026.
The profile applies to `POST /v1/systemone`; other endpoints display **N/A**.
Custom rates and the mode preference persist for live use. Offline demos start
with defaults and use synthetic usage.

The calculation uses reported `usage.input_tokens` and `usage.output_tokens`:
`(input_tokens × input_rate + output_tokens × output_rate) / 1,000,000`.
Missing output usage is acceptable when the output rate is zero. Missing required
usage displays **No usage**, and unfinished requests display **Pending**. These
rows are excluded from the total, which is labeled **Partial estimate**. Reported
usage on an error response still counts. Small positive costs never display as zero.

The relay publishes usage with every request summary after inspection finishes,
so totals do not depend on selecting requests. Capture-disabled, dropped, or
incomplete bodies may have no usage. Older relays need rebuilding to publish it.
Older saved sessions can recover usage from their complete captured responses;
summary-only sessions without usage remain unknown.

Saved sessions retain usage, and reopening recalculates estimates using the current
pricing profile. Rates are not a historical billing record. Estimates exclude any
fees or discounts outside this token profile and do not query your account balance.

## Hold the view while investigating

Click **Pause** to freeze the list. The relay continues forwarding and recording.
The paused banner reports how many requests arrived. A selected exchange can finish
updating while paused. Click **Resume** to load the latest retained list.

Click **Follow** to select the newest matching request and scroll to the top.
New requests keep the list at the top while Follow is on. Selecting a different
row or sorting turns Follow off.

New live rows appear in the opposite theme, then fade into the selected theme
over 1.2 seconds. Existing history and response updates do not repeat the cue.
The cue is disabled when the system requests reduced motion.

Click **Clear** to hide requests already in this window. **Restore** shows the ones
the relay still retains. Clear does not delete relay history or stop capture.

Click the star beside the exchange title to mark it. Stars remain while the relay
retains those exchanges. Save a session to keep the marked captures after eviction
or restart.

Drag the divider between Traffic and the details panel to resize them. Double-click
the divider to reset it. **Expand** gives the selected exchange the full workspace.
**Compact** reduces row height. The live app remembers theme, density, and the
divider width after a drag. Static `--demo` fixtures keep preferences in memory.
Short windows use a compact metrics strip to leave more room for captured bodies.
Diagrams fit the whole graph on first open. Manual pan and zoom stay in place
until you choose **Fit graph** or switch exchanges.

Diagram **Actual size** restores readable actual size; **Fit graph** shows the whole graph.
On the focused canvas, arrow keys pan, `+` and `-` zoom, `0` fits, and `1` restores
actual size.

## Explore JSON and compare exchanges

Request and Response provide indented, selectable JSON with Rune syntax colors and
line numbers. **Find** searches the complete captured JSON, with case-sensitive matches.
Use Enter and Shift+Enter for the next and previous match. Find from Tree opens
the code view at the result. Large documents use 16,000-byte display windows;
the footer arrows page through them, and Find brings its result into view.

**Tree** opens Rune's virtualized property tree. Arrow keys navigate the tree.
Use **Expand all**, **Collapse all**, or a row's disclosure arrow to explore it.
Select a property and click **Copy value** to copy its captured JSON value.

**Copy JSON** copies the complete captured JSON, even when the code preview is
limited. These are inspection copies after capture limits and redaction. They are
not the original wire bytes.

To compare two exchanges:

1. Select the first exchange and open **Compare**.
2. Click **Set baseline**.
3. Select another exchange.
4. Read the input and output changes in **Unified** or **Split** mode.

The baseline is an in-memory copy. It stays available if that row is evicted, but
opening a session, returning to live traffic, or observing a relay restart clears
it. The elapsed-time difference compares the two observations, not a controlled
performance benchmark.

## Save and reopen evidence

Click **Export** and choose a format. The operating system's Save dialog chooses
the destination.

| Format | Contents |
| --- | --- |
| Save session | Visible summaries, available request and response captures, and stars |
| Export CSV | Visible summaries, status, timestamps, elapsed time, sizes, and request IDs |
| Export exchange JSON | The selected exchange and its inspection copies |

Session export fetches available details from the local inspection API. It never
replays inference calls. If a detail was evicted, it uses an already-loaded copy
when available, or marks the exchange as summary-only. If the relay restarts during
export, the app reports an error and leaves the destination unchanged.

If the relay is unavailable, Save session preserves the summaries and captures
already received. The file, save confirmation, and reopened session identify this
as a **last-received snapshot**. Unavailable bodies remain summary-only. If a
refresh fails partway through saving, the app uses the cache from the start of
the save, rather than mixing it with partially refreshed data.

File operations run in the background. **Cancel** stops export before the final
write. Native file-dialog cancellation leaves the current view unchanged. Exports
write to a temporary file beside the destination before replacing it.

Click **Open** to inspect a saved session. Incoming live updates do not change the
opened session. **Return to live** reconnects the view to the current relay history.
Malformed or unsupported files leave the current investigation intact.

Body exports can contain private application data. Header credentials are absent
from the capture schema, but field-name redaction cannot detect secrets in arbitrary
prose. Files are plain JSON or CSV and use the destination directory's permissions.

## Use the keyboard

On macOS, use Command where this table says Ctrl.

| Shortcut | Action |
| --- | --- |
| Ctrl+K | Focus request search |
| Ctrl+F | Find in Request or Response; otherwise focus request search |
| Ctrl+1 through Ctrl+5 | Overview, Request, Response, Diagram, Compare |
| Ctrl+P | Pause or resume the live list |
| Ctrl+B | Star the selected exchange |
| Ctrl+L | Clear the view |
| Ctrl+E | Open export choices |
| Ctrl+O | Open a session |
| Alt+Up / Alt+Down | Previous or next request when the focused control passes the key through |
| F1 | Open help |
| Escape | Close the active dialog or JSON find field |

When the request table has focus, its native arrow, Home, End, and Page Up/Down
navigation also works. Tab visits controls throughout the app.

In a diagram card, Home and End scroll the focused properties region to its
first or last property. Page Up and Page Down move one viewport at a time.
Press Tab to enter a nested disclosure from the focused properties region;
arrow keys navigate the diagram card group.

## Display and file limits

The relay retains 100 exchanges by default, with a configurable maximum of 200.
Its capture limits remain unchanged. Code displays at most 16,000 UTF-8 bytes at a
time, with page navigation and character-safe boundaries.
JSON trees contain at most 4,096 nodes and 64 nested levels. Long tree labels are
shortened; Copy value retains the captured value. Comparisons use at most 16,000
bytes and 800 lines per body for rendering, with a visible notice when limited.
Equality checks use the complete captured JSON. Matching prefixes with different
tails are labeled as different beyond the preview, never unchanged.

Sessions accept at most 200 exchanges and 64 MB per file. A saved capture is limited
to 512 KB in its serialized representation. Find counts and navigates every match
across the complete captured JSON. It renders highlights only in the visible
preview window. The app does not automatically persist traffic.

## Repeat the visual checks

Build the workspace and run:

```sh
cargo build --workspace --locked
python tools/inspector_qa.py
```

The command renders the offline fixtures sequentially, including dark and light
themes, the minimum window size, JSON search and tree views, comparison, dialogs,
an error response, an empty view, and the expanded diagram. It also covers paused,
disconnected, saved and summary-only sessions, missing comparison bodies, invalid
queries, long JSON with Find at minimum size, export progress/cancellation/failure, long labels, and compact tree, split diff, and Help views.
Loading fixtures cover all five tabs. Other cases cover failed detail loading,
uncached disconnection, pending and empty responses, first use, empty saved
sessions, no search matches, and comparison before a baseline is set.
Use the [UI review checklist](ui-review-coverage.md) to audit states and transitions
that screenshots alone do not establish.
It writes PNGs, Rune element trees, logs,
and `report.json` to `artifacts/inspector-qa`. Inspect the PNGs for visual regressions.
The runner checks selected tabs and expected state markers in the accessibility
tree. These checks establish that scripts reached their intended screens; inspect
the PNGs to assess layout quality.

For one fixture or another build:

```sh
python tools/inspector_qa.py --case json-tree
python tools/inspector_qa.py --binary target/perf/jev-inspector --out artifacts/inspector-perf
```

On Windows, use `target/perf/jev-inspector.exe`. Build that binary first with
`cargo build --workspace --profile perf --locked`. Use `python3` on macOS if needed.
Compare the same build profile, scale, fixture, and hardware conditions. Reported
CPU times measure forced headless frames, including scripted transitions. They are
not interactive FPS, GPU completion times, or TypeSafe latency.

The Rust tests exercise Rune controls and file-dialog callbacks, real temporary
files, and a synthetic HTTP inspection API. This Windows run does not establish
macOS behavior or OS-level mouse, clipboard, and native-dialog integration.

## Keep investigation context

Filters keep the selected exchange open even when it leaves the result list.
**Show selected** clears the filters to reveal it. An incomplete query keeps the
last valid results until you finish editing. Clear view still works during editing.
Relay restarts and eviction of the selected exchange display a notice.
When selection is outside the filtered list, **Next request** enters its first
row and **Previous request** enters its last row. Alt+Down and Alt+Up do the same.

Saved pending exchanges are static snapshots labeled **Incomplete when saved**.
Summary-only sessions retain their timing, model, request ID, and size metadata.
Overview includes the returned choice among probability rows. Its disclosure
button counts the additional rows and remembers the expanded count for each
retained exchange. Long question lists use a scrollable selector above the
diagram. Truncated names retain their suffix and show the full name on hover.

Open **Help > Connection setup** for copyable relay, inspector, and backend
base URL commands. The same guide appears when a live session has no requests.
The commands share the inspector's connection file and relay port, including
custom paths. When a relay is connected, the guide labels its command for restart.
Connected setup starts with routing the backend and sending one request. The
restart command preserves a custom upstream and history limit. Reapply any custom
timeout before restarting.

Find reveals the matching text after you switch exchanges or return to Code.
It measures Unicode text with the same font used to render the JSON. Manual
scrolling remains available after the initial reveal.

Comparisons with differences outside the preview offer **Open current** and
**Copy baseline**. Copy baseline includes the complete captured JSON, even if the
original exchange has left retained history.

Help, Export, and the question menu contain keyboard input while open. Global
investigation shortcuts resume after you close them. Diagram's help button opens
its controls first. Probability labels use up to two decimal places; tiny values
and values close to 100% use bounds so rounding does not imply an exact endpoint.

Compare identifies incomplete or unavailable captures on both sides, including
the baseline. This context remains visible when expanded. Equality describes
captured JSON; an incomplete body can omit data. Narrow split comparisons suggest
Unified or Expand for longer lines.

Saved summary-only details describe a fixed snapshot, even if the live relay is
reconnecting. The footer retains the number of requests with captured JSON.
Diagram offers **Fit graph** to see relationships and **Actual size** to read cards.
