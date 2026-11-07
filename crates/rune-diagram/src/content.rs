use rune::{El, Key, ScrollPos, ScrollSpec, TextStyle, col, el, text};
use rune_kit::{Button, ButtonVariant, Kit};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Bounded disclosure state. Reuse a stable key for each independent card.
#[derive(Default)]
pub struct ContentState {
    expanded: BTreeSet<String>,
    pages: BTreeMap<String, usize>,
    scroll: BTreeMap<String, [f32; 4]>,
}
impl ContentState {
    pub fn toggle(&mut self, path: String) {
        if !self.expanded.remove(&path) && self.expanded.len() < 256 {
            self.expanded.insert(path);
        }
    }
    pub fn expanded(&self, path: &str) -> bool {
        self.expanded.contains(path)
    }
    pub fn visible(&self, path: &str) -> usize {
        self.pages.get(path).copied().unwrap_or(12)
    }
    pub fn more(&mut self, path: String) {
        self.more_by(path, 12);
    }
    pub fn more_by(&mut self, path: String, step: usize) {
        if self.pages.contains_key(&path) || self.pages.len() < 256 {
            let n = self.pages.entry(path).or_insert(12);
            *n = n.saturating_add(step).min(2048);
        }
    }
}

/// Lazily expanded JSON, with bounded text, depth and mounted rows.
pub struct DiagramContent<'a> {
    key: &'a str,
    value: Option<&'a Value>,
    keyboard: bool,
}
impl<'a> DiagramContent<'a> {
    pub fn new(key: &'a str, value: Option<&'a Value>) -> Self {
        Self {
            key,
            value,
            keyboard: true,
        }
    }
    pub(crate) fn keyboard(mut self, enabled: bool) -> Self {
        self.keyboard = enabled;
        self
    }
    pub fn build<S: 'static>(
        &self,
        state: &ContentState,
        get: impl Fn(&mut S) -> &mut ContentState + Copy + 'static,
        k: &Kit,
    ) -> El {
        self.build_with_activation(state, get, k, |_: &mut S| {})
    }
    pub(crate) fn build_with_activation<S: 'static>(
        &self,
        state: &ContentState,
        get: impl Fn(&mut S) -> &mut ContentState + Copy + 'static,
        k: &Kit,
        activate: impl Fn(&mut S) + Clone + 'static,
    ) -> El {
        let mut budget = 160;
        let content = match self.value {
            Some(v) => value::<S>(v, self.key, state, get, k, 0, &mut budget, self.keyboard),
            None => copy("Not captured", k),
        };
        let key = format!("{}:content", self.key);
        let watch_key = key.clone();
        let drawn_key = format!("{}:drawn", self.key);
        let watch_drawn_key = drawn_key.clone();
        let input_key = key.clone();
        let rows_key = format!("{}:rows", self.key);
        let mut pos = ScrollPos::default();
        if let (Some(v), Some(r)) = (state.scroll.get(&key), state.scroll.get(&rows_key)) {
            pos.viewport = [v[2], v[3]];
            pos.max[1] = (r[3] - v[3]).max(0.0);
            pos.offset[1] = (v[1] - r[1]).clamp(0.0, pos.max[1]);
        }
        let overflow = pos.max[1] > 0.5;
        // Rune drag deltas are screen points; layout and scroll offsets are
        // local points. Measure the effective transform, including parent zoom.
        let drawn_scale = state
            .scroll
            .get(&drawn_key)
            .filter(|_| pos.viewport[1] > 0.0)
            .map_or(1.0, |rect| rect[3] / pos.viewport[1])
            .max(0.001);
        let viewport = col()
            .key(key.clone())
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .on_layout(move |s: &mut S, rect| {
                let state = get(s);
                if state.scroll.len() >= 256 && !state.scroll.contains_key(&watch_key) {
                    state.scroll.clear();
                }
                state.scroll.insert(watch_key.clone(), rect);
            })
            .focusable(self.keyboard && overflow)
            // Keep keyboard focus legible even when Fit shrinks the card.
            .focus_ring(2.0 / drawn_scale.min(1.0), 0.0, k.c.text_base)
            .on_drawn_layout(move |s: &mut S, rect| {
                get(s).scroll.insert(watch_drawn_key.clone(), rect);
            })
            .on_key(move |_: &mut S, event, tx| {
                match event.key {
                    Key::PageDown => tx.scroll_page(input_key.clone(), 1),
                    Key::PageUp => tx.scroll_page(input_key.clone(), -1),
                    Key::Home => tx.scroll_to_y(input_key.clone(), 0.0),
                    Key::End => tx.scroll_to_bottom(input_key.clone()),
                    _ => return false,
                }
                // A descendant may be about to leave the viewport. Keep a
                // visible focus owner while paging, without activating a row.
                tx.focus_control(input_key.clone());
                true
            })
            .child(content.on_layout(move |s: &mut S, rect| {
                get(s).scroll.insert(rows_key.clone(), rect);
            }))
            .role("region")
            .label("Captured properties")
            .help("Page Up / Page Down scroll properties; Home / End jump to the first or last property");
        let mut area = col().grow(1.0).min_h(0.0).w_full().pr(12.0).child(viewport);
        if overflow {
            let track = (pos.viewport[1] - 4.0).max(1.0);
            let height = (track * pos.viewport[1] / (pos.viewport[1] + pos.max[1]))
                .clamp(16.0_f32.min(track), track);
            let travel = track - height;
            let top = 2.0 + travel * pos.offset[1] / pos.max[1];
            let page_key = key.clone();
            let activate_track = activate.clone();
            area = area.child(
                el().key(format!("{}:scrollbar", self.key))
                    .pin(None, Some(0.0), Some(0.0), Some(0.0))
                    .w(10.0)
                    .fill(k.c.layer_02)
                    .a11y_hidden()
                    .on_drag(move |s: &mut S, d, tx| {
                        if d.phase == rune::Phase::Down {
                            activate_track(s);
                            tx.focus_control(page_key.clone());
                            tx.scroll_page(page_key.clone(), if d.y < top { -1 } else { 1 });
                        }
                    })
                    .child(
                        el().key(format!("{}:scroll-thumb", self.key))
                            .absolute(2.0, top)
                            .size(6.0, height)
                            .radius(3.0)
                            .fill(k.c.text_faint)
                            .hover(k.c.text_muted)
                            .on_drag(move |s: &mut S, d, tx| {
                                if d.cancelled() || travel <= 0.0 {
                                    return;
                                }
                                if d.phase == rune::Phase::Down {
                                    d.data = pos.offset[1];
                                    activate(s);
                                    tx.focus_control(key.clone());
                                }
                                tx.scroll_to_instant(
                                    key.clone(),
                                    0.0,
                                    (d.data + d.dy / drawn_scale * pos.max[1] / travel)
                                        .clamp(0.0, pos.max[1]),
                                );
                                if d.phase != rune::Phase::Down {
                                    tx.live_only();
                                }
                            }),
                    ),
            );
        }
        area
    }
}

fn copy(value: impl Into<String>, k: &Kit) -> El {
    text(value)
        .w_full()
        .wrap_anywhere()
        .font(TextStyle::new(12.0, 400.0))
        .color(k.c.text_base)
        .selectable(true)
}

// JSON Pointer escaping keeps a key containing '/' distinct from nested properties.
fn child_path(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

#[allow(clippy::too_many_arguments)]
fn value<S: 'static>(
    v: &Value,
    path: &str,
    state: &ContentState,
    get: impl Fn(&mut S) -> &mut ContentState + Copy + 'static,
    k: &Kit,
    depth: usize,
    budget: &mut usize,
    keyboard: bool,
) -> El {
    if *budget == 0 || depth > 8 {
        return copy("Preview limit reached", k);
    }
    *budget -= 1;
    let mut out = col().w_full().gap(6.0);
    if v.is_object() || v.is_array() {
        let count = v
            .as_object()
            .map(|o| o.len())
            .or_else(|| v.as_array().map(|a| a.len()))
            .unwrap_or(0);
        if count == 0 {
            return copy(
                if v.is_array() {
                    "[] Empty array"
                } else {
                    "{} Empty object"
                },
                k,
            );
        }
        let limit = state.visible(path).min(count);
        // Enumerate only the visible page; unopened subtrees are never rendered.
        let entries: Box<dyn Iterator<Item = (String, &Value)> + '_> = match v {
            Value::Object(o) => Box::new(o.iter().take(limit).map(|(n, v)| (n.clone(), v))),
            Value::Array(a) => Box::new(
                a.iter()
                    .take(limit)
                    .enumerate()
                    .map(|(i, v)| (i.to_string(), v)),
            ),
            _ => unreachable!(),
        };
        for (name, child) in entries {
            if *budget == 0 {
                out = out.child(copy("Preview limit reached", k));
                break;
            }
            let child_key = child_path(path, &name);
            let label = if name.is_empty() { "\"\"" } else { &name };
            if child.is_object() || child.is_array() {
                let expanded = state.expanded(&child_key);
                let count = child
                    .as_object()
                    .map(|o| o.len())
                    .or_else(|| child.as_array().map(|a| a.len()))
                    .unwrap_or(0);
                let button_key = format!("{child_key}:toggle");
                let action = child_key.clone();
                out = out.child(
                    Button::new(format!(
                        "{} {} ({count})",
                        if expanded { "−" } else { "+" },
                        label
                    ))
                    .variant(ButtonVariant::Ghost)
                    .on_press(move |s: &mut S| get(s).toggle(action.clone()))
                    .build(k)
                    .focusable(keyboard)
                    .expanded(expanded)
                    .key(button_key),
                );
                *budget -= 1;
                if expanded {
                    out = out.child(
                        value::<S>(
                            child,
                            &child_key,
                            state,
                            get,
                            k,
                            depth + 1,
                            budget,
                            keyboard,
                        )
                        .pl(10.0),
                    );
                }
            } else {
                out = out
                    .child(
                        text(label)
                            .font(TextStyle::new(12.0, 400.0).mono())
                            .color(k.c.text_muted),
                    )
                    .child(value::<S>(
                        child,
                        &child_key,
                        state,
                        get,
                        k,
                        depth + 1,
                        budget,
                        keyboard,
                    ));
            }
        }
        if count > limit {
            let action = path.to_string();
            out = out.child(
                Button::new(format!("Show more ({} remaining)", count - limit))
                    .variant(ButtonVariant::Ghost)
                    .on_press(move |s: &mut S| get(s).more(action.clone()))
                    .build(k)
                    .focusable(keyboard)
                    .key(format!("{path}:more")),
            );
        }
    } else {
        let raw = match v {
            Value::String(s) if s.is_empty() => "Empty string".into(),
            Value::String(s) => s.clone(),
            _ => v.to_string(),
        };
        let expanded = state.expanded(path);
        let limit = if expanded { 4000 } else { 140 };
        let preview: String = raw.chars().take(limit).collect();
        let long = raw.chars().count() > 140;
        out = out.child(copy(
            format!(
                "{preview}{}",
                if raw.chars().count() > limit {
                    "…"
                } else {
                    ""
                }
            ),
            k,
        ));
        if long {
            let action = path.to_string();
            out = out.child(
                Button::new(if expanded {
                    "Less text"
                } else {
                    "More text (up to 4,000 characters)"
                })
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut S| get(s).toggle(action.clone()))
                .build(k)
                .focusable(keyboard)
                .key(format!("{path}:text")),
            );
        }
    }
    out
}
