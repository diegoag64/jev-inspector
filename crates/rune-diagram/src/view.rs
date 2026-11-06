use crate::theme::{alpha, percent};
use crate::{ContentState, DiagramContent, DiagramState, Graph, Node, NodeBody, NodeStatus};
use rune::{
    Color, El, Key, LineCap, Path, Phase, ScrollSpec, Shape, TextStyle, canvas, col, el, row, text,
};
use rune_kit::{Button, ButtonVariant, Kit};

fn label(value: impl Into<String>, size: f32, color: Color) -> El {
    text(value).font(TextStyle::new(size, 400.0)).color(color)
}

/// A graph with a camera toolbar, keyboard navigation, pan, wheel zoom and pinch.
pub struct Diagram<'a, S> {
    key: &'a str,
    graph: &'a Graph,
    state: &'a DiagramState,
    get: fn(&mut S) -> &mut DiagramState,
    minimum_size: [f32; 2],
    readable_focus: Option<&'a str>,
}
impl<'a, S: 'static> Diagram<'a, S> {
    pub fn new(
        key: &'a str,
        graph: &'a Graph,
        state: &'a DiagramState,
        get: fn(&mut S) -> &mut DiagramState,
    ) -> Self {
        Self {
            key,
            graph,
            state,
            get,
            minimum_size: [0.0, 0.0],
            readable_focus: None,
        }
    }
    /// Reserve whitespace around the caller's layout without adding fake nodes.
    pub fn content_size(mut self, size: [f32; 2]) -> Self {
        if size
            .into_iter()
            .all(|v| v.is_finite() && (0.0..=100_000.0).contains(&v))
        {
            self.minimum_size = size;
        }
        self
    }
    /// When fitting would shrink text below 75%, start on this node at actual
    /// size. Fit still shows the whole graph; manual pan/zoom survives resizing.
    pub fn readable_focus(mut self, node: &'a str) -> Self {
        self.readable_focus = Some(node);
        self
    }
    pub fn build(self, k: &Kit) -> El {
        if let Err(message) = self.graph.validate() {
            return label(message, 13.0, k.c.danger.fg);
        }
        if self.graph.nodes.is_empty() {
            return label("No diagram data captured.", 13.0, k.c.text_muted);
        }
        let get = self.get;
        let bounds = self.graph.size();
        let size = [
            bounds[0].max(self.minimum_size[0]),
            bounds[1].max(self.minimum_size[1]),
        ];
        let focus = self
            .readable_focus
            .and_then(|id| self.graph.nodes.iter().find(|n| n.id == id))
            .map(|n| n.rect);
        let cam = if self.state.fitted {
            self.state.camera
        } else {
            self.state.initial_camera(size, focus)
        };
        let initialize = move |s: &mut S| {
            let state = get(s);
            if !state.fitted {
                state.initialize(size, focus);
            }
        };
        let toolbar = row()
            .gap(6.0)
            .center_y()
            .child(
                Button::new("−")
                    .on_press(move |s: &mut S| {
                        initialize(s);
                        get(s).zoom(1.0 / 1.2);
                    })
                    .build(k)
                    .key(format!("{}:zoom-out", self.key))
                    .label("Zoom diagram out"),
            )
            .child(label(format!("{:.0}%", cam.scale * 100.0), 12.0, k.c.text_muted).w(45.0))
            .child(
                Button::new("+")
                    .on_press(move |s: &mut S| {
                        initialize(s);
                        get(s).zoom(1.2);
                    })
                    .build(k)
                    .key(format!("{}:zoom-in", self.key))
                    .label("Zoom diagram in"),
            )
            .child(
                Button::new("100%")
                    .on_press(move |s: &mut S| {
                        initialize(s);
                        let state = get(s);
                        state.zoom(1.0 / state.camera.scale);
                    })
                    .build(k)
                    .key(format!("{}:actual-size", self.key))
                    .help("Read cards at actual size; drag or use arrow keys to pan"),
            )
            .child(
                Button::new("Fit")
                    .on_press(move |s: &mut S| get(s).fit(size))
                    .build(k)
                    .key(format!("{}:fit", self.key)),
            );
        let mut lines = Shape::new();
        let mut edge_labels = Vec::new();
        for edge in &self.graph.edges {
            let from = &self
                .graph
                .nodes
                .iter()
                .find(|n| n.id == edge.from)
                .expect("validated edge")
                .rect;
            let to = &self
                .graph
                .nodes
                .iter()
                .find(|n| n.id == edge.to)
                .expect("validated edge")
                .rect;
            let a = [from[0] + from[2], from[1] + from[3] / 2.0];
            let b = [to[0], to[1] + to[3] / 2.0];
            let bend = ((b[0] - a[0]).abs() * 0.5).max(40.0);
            let path = Path::new()
                .move_to(a)
                .cubic_to([a[0] + bend, a[1]], [b[0] - bend, b[1]], b);
            let color = if edge.active {
                k.c.text_faint
            } else {
                k.c.border_muted
            };
            lines = lines.stroke_path(&path, 1.0, LineCap::Round, color);
            if let Some(caption) = &edge.label {
                edge_labels.push(
                    text(caption)
                        .font(TextStyle::new(12.0, 400.0).mono())
                        .color(k.c.text_muted)
                        .absolute((a[0] + b[0]) / 2.0 - 24.0, (a[1] + b[1]) / 2.0 - 22.0),
                );
            }
        }
        let mut world = el()
            .size(size[0], size[1])
            .opacity(if self.state.viewport[0] > 0.0 {
                1.0
            } else {
                0.0
            })
            .absolute(0.0, 0.0)
            .scale(cam.scale)
            // Rune scales around the center; our camera uses the top-left.
            .offset(
                cam.x - size[0] * (1.0 - cam.scale) / 2.0,
                cam.y - size[1] * (1.0 - cam.scale) / 2.0,
            )
            .child(canvas(lines).absolute(0.0, 0.0).size(size[0], size[1]));
        for caption in edge_labels {
            world = world.child(caption);
        }
        for node in &self.graph.nodes {
            let key = format!("{}:node:{}:{}", self.key, node.id.len(), node.id);
            world = world.child(
                DiagramCard::new(&key, node)
                    .build(self.state, get, k)
                    .absolute(node.rect[0], node.rect[1]),
            );
        }
        let mut grid = Shape::new();
        let [w, h] = self.state.viewport;
        // useDiagramViewport.ts thins the decorative grid at overview zooms.
        let spacing =
            (24.0 * cam.scale.max(1.0 / cam.scale.sqrt())).max(((w * h) / 12_000.0).sqrt());
        for x in 0..((w / spacing).ceil() as usize).min(250) {
            for y in 0..((h / spacing).ceil() as usize).min(250) {
                let p = [
                    x as f32 * spacing + cam.x.rem_euclid(spacing),
                    y as f32 * spacing + cam.y.rem_euclid(spacing),
                ];
                grid = grid.line(
                    p,
                    p,
                    1.6 * cam.scale.sqrt().clamp(0.5, 0.75),
                    LineCap::Round,
                    alpha(k.c.layer_02, 0.45 * cam.scale.sqrt().min(1.0)),
                );
            }
        }
        let viewport = el().key(format!("{}:viewport", self.key)).w_full().grow(1.0).min_h(0.0)
            .fill(k.c.layer_01).radius(8.0).clip_rounded(8.0).border(1.0,k.c.border_base)
            .focusable(true).role("group").label("Diagram canvas")
            .description("Drag background to pan. Wheel or pinch to zoom. Arrow keys pan, plus and minus zoom, zero fits, one sets actual size.")
            .on_layout_tx(move |s: &mut S, [_,_,w,h], tx| {
                let state=get(s);
                let changed = (state.viewport[0]-w).abs() > 0.5 || (state.viewport[1]-h).abs() > 0.5;
                state.viewport=[w,h];
                if !state.fitted || (changed && !state.manual_view) {
                    if !state.fitted || state.initial_view { state.initialize(size, focus); }
                    else { state.fit(size); }
                    tx.instant();
                }
            })
            .on_drag(move |s: &mut S, d, _| {
                initialize(s); let state = get(s); state.manual_view = true;
                match d.phase {
                    Phase::Down => state.drag_start=state.camera,
                    _ => { state.camera=state.drag_start; if !d.cancelled() { state.camera.pan(d.dx,d.dy); } }
                }
            })
            .on_wheel(move |s: &mut S, e, _| {
                initialize(s); get(s).manual_view = true; get(s).camera.zoom((-e.dy/240.0).clamp(-2.0,2.0).exp(), [e.x,e.y], [e.x,e.y]); true
            })
            .on_magnify(move |s: &mut S, e, _| {
                initialize(s); get(s).manual_view = true; get(s).camera.zoom((1.0+e.delta).max(0.01), [e.x,e.y], [e.x,e.y]); true
            })
            .on_key(move |s: &mut S, e, _| {
                initialize(s); let state=get(s); let was_manual = state.manual_view; state.manual_view = true;
                match &e.key {
                    Key::Left => state.camera.pan(40.0,0.0), Key::Right => state.camera.pan(-40.0,0.0),
                    Key::Up => state.camera.pan(0.0,40.0), Key::Down => state.camera.pan(0.0,-40.0),
                    Key::Text(t) if t=="+" || t=="=" => state.zoom(1.2),
                    Key::Text(t) if t=="-" => state.zoom(1.0/1.2),
                    Key::Text(t) if t=="0" => state.fit(size),
                    Key::Text(t) if t=="1" => state.zoom(1.0 / state.camera.scale), _ => { state.manual_view = was_manual; return false; },
                } true
            }).child(canvas(grid).absolute(0.0,0.0).size(w.max(1.0),h.max(1.0))).child(world);
        col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .gap(8.0)
            .child(toolbar)
            .child(viewport)
            .child(
                label(
                    "Drag / arrows to pan · Scroll / + / − to zoom · 0 to fit · 1 for readable actual size",
                    11.0,
                    k.c.text_muted,
                )
                .wrap()
                .px(12.0)
                .py(3.0),
            )
    }
}

/// A selectable card, independently usable outside the diagram viewport.
pub struct DiagramCard<'a> {
    key: &'a str,
    node: &'a Node,
}
impl<'a> DiagramCard<'a> {
    pub fn new(key: &'a str, node: &'a Node) -> Self {
        Self { key, node }
    }
    pub fn build<S: 'static>(
        &self,
        state: &DiagramState,
        get: fn(&mut S) -> &mut DiagramState,
        k: &Kit,
    ) -> El {
        let n = self.node;
        let selected = state.selected.as_deref() == Some(&n.id);
        let color = if n.status == NodeStatus::Error {
            k.c.danger.fg
        } else {
            k.c.text_accent
        };
        let workflow = n.style == crate::CardStyle::Workflow;
        let content_card = !workflow && matches!(n.body, NodeBody::Json(_));
        let radius = if workflow { 12.0 } else { 14.0 };
        let fill = if workflow {
            if n.status == NodeStatus::Active {
                k.c.layer_02
            } else {
                k.c.layer_01
            }
        } else {
            k.c.bg_base
        };
        let border = if workflow {
            match n.status {
                NodeStatus::Success | NodeStatus::Active => k.c.text_accent,
                NodeStatus::Error => k.c.danger.fg,
                NodeStatus::Alternative | NodeStatus::Unavailable => k.c.border_muted,
                _ => k.c.text_faint,
            }
        } else {
            k.c.border_strong
        };
        let id = n.id.clone();
        let heading = col()
            .gap(if content_card { 5.0 } else { 7.0 })
            .w_full()
            .focusable(true)
            .role("button")
            .label(format!("Select {}", n.title))
            .key(format!("{}:select", self.key))
            .on_press(move |s: &mut S| get(s).selected = Some(id.clone()))
            .child(
                text(&n.kind)
                    .font(TextStyle::new(12.0, 500.0).mono())
                    .color(color),
            )
            .child(
                text(&n.title)
                    .font(TextStyle::new(
                        if content_card { 14.0 } else { 16.0 },
                        600.0,
                    ))
                    .color(k.c.text_base),
            );
        let body = match &n.body {
            NodeBody::Result(result) => result.build(
                self.key,
                &state.content,
                move |s: &mut S| &mut get(s).content,
                k,
            ),
            NodeBody::Text(t) => col()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    label(t, 13.0, k.c.text_base)
                        .w_full()
                        .wrap_anywhere()
                        .selectable(true),
                ),
            NodeBody::Json(v) => DiagramContent::new(self.key, v.as_ref()).build(
                &state.content,
                move |s: &mut S| &mut get(s).content,
                k,
            ),
            NodeBody::Probabilities { values, selected } => ProbabilityBars::new(
                self.key,
                values,
                selected.as_deref(),
            )
            .build(&state.content, move |s: &mut S| &mut get(s).content, k),
        };
        col()
            .key(self.key)
            .size(n.rect[2], n.rect[3])
            .padding(if content_card || workflow { 14.0 } else { 16.0 })
            .gap(if content_card { 8.0 } else { 10.0 })
            .fill(fill)
            .radius(radius)
            .clip_rounded(radius)
            .opacity(if n.status == NodeStatus::Alternative {
                0.65
            } else {
                1.0
            })
            .border(
                if selected {
                    2.0
                } else if workflow {
                    1.0
                } else {
                    1.3
                },
                if selected { k.c.border_focus } else { border },
            )
            .child(heading)
            .child(body)
    }
}

/// Percent labels use six significant digits, matching Jev's probability view.
pub struct ProbabilityBars<'a> {
    key: &'a str,
    values: &'a [(String, f64)],
    selected: Option<&'a str>,
}
impl<'a> ProbabilityBars<'a> {
    pub fn new(key: &'a str, values: &'a [(String, f64)], selected: Option<&'a str>) -> Self {
        Self {
            key,
            values,
            selected,
        }
    }
    pub fn build<S: 'static>(
        &self,
        state: &ContentState,
        get: impl Fn(&mut S) -> &mut ContentState + Copy + 'static,
        k: &Kit,
    ) -> El {
        col()
            .grow(1.0)
            .min_h(0.0)
            .w_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(self.rows(state, get, k))
    }
    pub(crate) fn rows<S: 'static>(
        &self,
        state: &ContentState,
        get: impl Fn(&mut S) -> &mut ContentState + Copy + 'static,
        k: &Kit,
    ) -> El {
        let mut out = col().gap(17.0).w_full();
        if self.values.is_empty() {
            out = out.child(label("Probabilities not captured", 12.0, k.c.text_muted));
        }
        let limit = state.visible(self.key);
        for (name, p) in self.values.iter().take(limit) {
            let chosen = self.selected.is_none() || self.selected == Some(name.as_str());
            let color = if chosen {
                k.c.text_accent
            } else {
                alpha(k.c.text_accent, 0.35)
            };
            out = out.child(
                col()
                    .gap(9.0)
                    .child(
                        row()
                            .gap(8.0)
                            .child(label(name, 15.0, k.c.text_base).grow(1.0))
                            .child(
                                text(percent(*p))
                                    .font(TextStyle::new(13.0, 400.0).mono())
                                    .color(k.c.text_base),
                            ),
                    )
                    .child(
                        el().w_full()
                            .h(7.0)
                            .fill(alpha(k.c.text_base, 0.1))
                            .radius(4.0)
                            .child(
                                el().absolute(0.0, 0.0)
                                    .h(7.0)
                                    .w_frac(if p.is_finite() {
                                        p.clamp(0.0, 1.0) as f32
                                    } else {
                                        0.0
                                    })
                                    .fill(color)
                                    .radius(4.0),
                            ),
                    ),
            );
        }
        if self.values.len() > limit {
            let key = self.key.to_string();
            out = out.child(
                Button::new(format!(
                    "Show more ({} remaining)",
                    self.values.len() - limit
                ))
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| get(s).more(key.clone()))
                .build(k)
                .key(format!("{}:more", self.key)),
            );
        }
        if let Some(selected) = self.selected {
            out = out.child(label(format!("Selected: {selected}"), 14.0, k.c.text_base).pt(7.0));
        }
        out
    }
}
