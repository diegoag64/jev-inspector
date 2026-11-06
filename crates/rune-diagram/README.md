# rune-diagram

Native Rune diagram components adapted from the MIT-licensed
[jev-devtools](https://github.com/diegoag64/jev-devtools) components at revision
`b6ccfdd83729235fbfa0ddbea77d89166621fccc`.

The port follows `diagram-camera.ts`, `DiagramContent.tsx`, `TraceDiagram.tsx`,
and the card/connection structure in `FlowOverview.tsx`. It uses Rune elements and
vector paths; there is no browser, React runtime, or JavaScript dependency.

## Try the components

From the containing Cargo workspace:

```sh
cargo run -p rune-diagram --example flow
```

The example shows a synthetic support workflow with explicit selected and
unselected branches. It performs no network requests.

Drag the background to pan. Use the mouse wheel, trackpad pinch, or +/− buttons
to zoom. Fit restores the overview. With the canvas focused, arrow keys pan,
+/− zoom, and 0 fits. Click card headings to select a node. Nested JSON objects
expand inside their own scrollable cards; scrolling inside a card scrolls its
content instead of zooming the canvas.

## Component API

| Type | Purpose |
| --- | --- |
| `Graph`, `Node`, `Edge` | Caller-supplied IDs, geometry, content, status, and connections |
| `DiagramState` | Camera, selection, and disclosure state owned by the application |
| `Diagram` | Viewport, controls, connections, and positioned cards |
| `DiagramCard` | A selectable card usable independently of the viewport |
| `DiagramContent` | Lazy JSON properties, text expansion, and pagination |
| `ProbabilityBars` | Returned probabilities and an optional explicit selected category |
| `ResultContent` | Typed result presentation with probability bars, a large metric, and confidence |
| `jev_theme` | Original Jev dark colors and a matching light adaptation |
| `CardStyle` | Bright trace cards or status-colored workflow cards |
| `Camera` | Fit and anchored zoom calculations independent of the UI |

Store `DiagramState` in the app state, build a `Graph`, and return:

```rust,ignore
Diagram::new("workflow", &graph, &state.diagram, |s: &mut State| {
    &mut s.diagram
}).build(&kit)
```

Give each mounted diagram a unique key and its own state. The parent must have
a bounded height because the diagram grows into its available space. Node
rectangles are `[x, y, width, height]` in content coordinates. Connections run
from the source card's right midpoint to the destination's left midpoint.
`Diagram::content_size()` reserves whitespace around an authored layout.
`DiagramState::request_fit()` refits after a resize without clearing disclosures.
`DiagramState::reset()` clears selection, expansion, and the camera when changing
documents. Automatic fitting tracks viewport changes; manual pan and zoom survive
resizing. Fit recalculates the camera for the settled viewport.

Use `Diagram::readable_focus("node-id")` to start on a specific card at actual size
when fitting the entire graph would shrink it below 75%. Fit still shows the whole
graph. The inspector uses this option for the answer card in compact panes.

`NodeBody::Json(None)` displays “Not captured.” `Some(Value::Null)` displays the
captured value `null`. Active edges and selected probability categories are
explicit caller inputs; the component never infers workflow execution or model
reasoning from a probability distribution.

## Bounds and current scope

- Invalid graphs display an error. Graphs support up to 256 nodes and 1,024 edges.
- JSON previews mount at most 160 entries per card, expand at most eight nested
  levels, page 12 properties at a time, and retain at most 256 disclosure/page keys.
- Text previews show 140 characters initially and up to 4,000 when expanded.
- Probability bars page 12 values at a time. Labels use compact percentages with
  six significant digits; graph values remain unchanged. Only bar geometry clamps to [0,1].
- Camera scale ranges from 10% to 400%. Fit never enlarges a diagram beyond 100%.

This first port renders caller-positioned graphs. It does not yet include automatic
workflow layout, node dragging, edge editing, animated flow dots, embedded image
previews, or Jev collection/loop execution controls. Pan and zoom currently rebuild
the bounded element tree; they do not yet use Rune's live camera bindings.

The application is responsible for bounding and redacting the source data it
hands to the component. Preview limits do not limit the size of an owned `Graph`.

## Attribution

The request view follows the original `TraceDiagram.tsx` coordinates: 240-point
context cards at x=20, a 320-point answer card at x=390, and 230-point output cards
at x=870. Its dark palette, 14-point corner radius, 1.3-point outlines, magenta
eyebrows, probability tracks, and muted curves follow `inspector.css`. Grid spacing
and opacity follow `useDiagramViewport.ts`. The original stylesheet is dark-only;
the light palette is a native adaptation. Font rasterization and toolbar controls
remain platform-specific. `CardStyle::Workflow` follows `FlowOverview` card colors
and status borders instead of the trace card's bright outline.

Camera behavior, JSON disclosure behavior, and the diagram presentation are
adapted from jev-devtools, copyright 2026 Diego Arias. Its MIT license is retained
in [LICENSE](LICENSE). Rune remains a separate dependency under its own license.
