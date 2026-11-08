use crate::{ContentState, DiagramContent, DiagramState, Node, NodeBody};
use rune::{El, Key, ScrollSpec, TextStyle, col, el, row, text};
use rune_kit::{Button, ButtonVariant, IconButton, Kit, syntax};
use serde_json::{Value, json};

/// A reader inside the diagram, independent of the graph camera and card previews.
pub struct InspectionState {
    pub node: String,
    tree: bool,
    page: usize,
    copied: bool,
    content: ContentState,
    viewport: [f32; 4],
    rows: [f32; 4],
}
impl InspectionState {
    pub(crate) fn new(node: String) -> Self {
        Self {
            node,
            tree: false,
            page: 0,
            copied: false,
            content: ContentState::default(),
            viewport: [0.0; 4],
            rows: [0.0; 4],
        }
    }
}

fn value(node: &Node) -> Option<Value> {
    node.inspection.clone().or_else(|| match &node.body {
        NodeBody::Json(value) => value.clone(),
        NodeBody::Text(value) => Some(Value::String(value.clone())),
        NodeBody::Probabilities { values, selected } => {
            Some(json!({"selected":selected,"probabilities":values}))
        }
        NodeBody::Result(result) => Some(
            json!({"selected":result.selected,"headline":result.headline,
            "caption":result.caption,"details":result.details,"confidence":result.confidence,
            "likelihood":result.likelihood,"probabilities":result.probabilities}),
        ),
    })
}

// Page large captures without dropping bytes or splitting UTF-8 characters.
fn pages(source: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < source.len() {
        let mut end = (start + 16_000).min(source.len());
        while !source.is_char_boundary(end) {
            end -= 1;
        }
        ranges.push(start..end);
        start = end;
    }
    if ranges.is_empty() {
        ranges.push(0..0);
    }
    ranges
}

pub(crate) fn view<S: 'static>(
    key: &str,
    node: &Node,
    state: &DiagramState,
    get: fn(&mut S) -> &mut DiagramState,
    k: &Kit,
) -> El {
    let inspection = state.inspection.as_ref().expect("open inspector");
    let root_key = format!("{key}:inspector");
    let scroll_key = format!("{root_key}:scroll");
    let return_key = format!("{key}:inspect");
    let escape_return = return_key.clone();
    let data = value(node);
    let source = data
        .as_ref()
        .map(|v| serde_json::to_string_pretty(v).expect("JSON value"))
        .unwrap_or_else(|| "No captured value is available for this card.".into());
    let ranges = pages(&source);
    let page = inspection.page.min(ranges.len() - 1);
    let range = ranges[page].clone();
    let preview = &source[range.clone()];
    let copy = source.clone();
    let mut header = row()
        .center_y()
        .gap(8.0)
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(3.0)
                .child(
                    text(&node.kind)
                        .font(TextStyle::new(10.0, 500.0))
                        .color(k.c.text_muted)
                        .truncate(),
                )
                .child(
                    text(&node.title)
                        .font(TextStyle::new(15.0, 600.0))
                        .color(k.c.text_base)
                        .truncate_middle()
                        .help(&node.title),
                ),
        )
        .child(
            IconButton::new(&rune_kit::glyphs::CLOSE)
                .label("Close data inspector")
                .on_press_tx(move |s: &mut S, tx| {
                    get(s).inspection = None;
                    tx.focus_control(return_key.clone());
                })
                .build(k)
                .key(format!("{root_key}:close")),
        );
    header = header.pb(10.0);
    let controls = row()
        .center_y()
        .gap(6.0)
        .child(
            Button::new("JSON")
                .variant(if inspection.tree {
                    ButtonVariant::Ghost
                } else {
                    ButtonVariant::Contrast
                })
                .on_press(move |s: &mut S| {
                    if let Some(v) = &mut get(s).inspection {
                        v.tree = false;
                    }
                })
                .build(k)
                .key(format!("{root_key}:json"))
                .role("tab")
                .focus_selected(!inspection.tree),
        )
        .child(
            Button::new("Tree")
                .variant(if inspection.tree {
                    ButtonVariant::Contrast
                } else {
                    ButtonVariant::Ghost
                })
                .on_press(move |s: &mut S| {
                    if let Some(v) = &mut get(s).inspection {
                        v.tree = true;
                    }
                })
                .build(k)
                .key(format!("{root_key}:tree"))
                .role("tab")
                .focus_selected(inspection.tree),
        )
        .child(el().grow(1.0))
        .child(
            Button::new(if inspection.copied { "Copied" } else { "Copy" })
                .icon(&rune_kit::glyphs::COPY)
                .disabled(data.is_none())
                .help("Copy the full captured value as JSON")
                .on_press_tx(move |s: &mut S, tx| {
                    tx.copy(copy.clone());
                    if let Some(v) = &mut get(s).inspection {
                        v.copied = true;
                    }
                })
                .build(k)
                .key(format!("{root_key}:copy")),
        );
    let body = if inspection.tree {
        col().grow(1.0).min_h(0.0).padding(10.0).child(
            DiagramContent::new(&format!("{root_key}:tree-data"), data.as_ref()).build(
                &inspection.content,
                move |s: &mut S| {
                    &mut get(s)
                        .inspection
                        .as_mut()
                        .expect("mounted inspector")
                        .content
                },
                k,
            ),
        )
    } else {
        let input_key = scroll_key.clone();
        let spans =
            syntax::SyntaxTheme::from_kit(k).spans(&syntax::highlight(Some("json"), preview), 0);
        let viewport = col()
            .key(&scroll_key)
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .focusable(true)
            .focus_ring(1.0, -1.0, k.c.text_faint)
            .role("region")
            .label("Card JSON")
            .on_layout(move |s: &mut S, rect| {
                if let Some(v) = &mut get(s).inspection {
                    v.viewport = rect;
                }
            })
            .on_key(move |_: &mut S, e, tx| {
                match e.key {
                    Key::Home => tx.scroll_to_y(input_key.clone(), 0.0),
                    Key::End => tx.scroll_to_bottom(input_key.clone()),
                    Key::PageUp => tx.scroll_page(input_key.clone(), -1),
                    Key::PageDown => tx.scroll_page(input_key.clone(), 1),
                    _ => return false,
                }
                true
            })
            .child(
                text(preview)
                    .font(TextStyle::new(13.0, 400.0).mono())
                    .spans(spans)
                    .color(k.c.text_base)
                    .wrap_anywhere()
                    .selectable(true)
                    .w_full()
                    .pl(10.0)
                    .pr(10.0)
                    .py(10.0)
                    .on_layout(move |s: &mut S, rect| {
                        if let Some(v) = &mut get(s).inspection {
                            v.rows = rect;
                        }
                    }),
            );
        let mut area = col()
            .key(format!("{root_key}:code-area:{page}"))
            .grow(1.0)
            .min_h(0.0)
            .w_full()
            .pr(24.0)
            .child(viewport);
        let v = inspection.viewport;
        let r = inspection.rows;
        let max = (r[3] - v[3]).max(0.0);
        if max > 0.5 && v[3] > 4.0 {
            let track = v[3] - 4.0;
            let height = (track * v[3] / r[3]).clamp(24.0_f32.min(track), track);
            let travel = track - height;
            let offset = (v[1] - r[1]).clamp(0.0, max);
            let top = 2.0 + travel * offset / max;
            let page_key = scroll_key.clone();
            let thumb_key = scroll_key.clone();
            area = area.child(
                el().key(format!("{root_key}:scrollbar"))
                    .pin(None, Some(0.0), Some(0.0), Some(0.0))
                    .w(12.0)
                    .radius(6.0)
                    .fill(k.c.layer_02)
                    .a11y_hidden()
                    .on_drag(move |_: &mut S, d, tx| {
                        if d.phase == rune::Phase::Down {
                            tx.scroll_to_y(
                                page_key.clone(),
                                (offset + if d.y < top { -v[3] } else { v[3] }).clamp(0.0, max),
                            );
                            tx.focus_control(page_key.clone());
                        }
                    })
                    .child(
                        el().key(format!("{root_key}:scroll-thumb"))
                            .absolute(2.0, top)
                            .size(8.0, height)
                            .radius(4.0)
                            .fill(k.c.text_faint)
                            .hover(k.c.text_muted)
                            .on_drag(move |_: &mut S, d, tx| {
                                if d.cancelled() || travel <= 0.0 {
                                    return;
                                }
                                if d.phase == rune::Phase::Down {
                                    d.data = offset;
                                    tx.focus_control(thumb_key.clone());
                                }
                                tx.scroll_to_y(
                                    thumb_key.clone(),
                                    (d.data + d.dy * max / travel).clamp(0.0, max),
                                );
                            }),
                    ),
            );
        }
        area
    };
    let mut footer = row().center_y().gap(6.0).child(
        text(if inspection.tree {
            "Tree preview · JSON and Copy include the full value".into()
        } else {
            format!(
                "{} bytes · Page {} / {}",
                source.len(),
                page + 1,
                ranges.len()
            )
        })
        .font(TextStyle::new(10.0, 400.0))
        .color(k.c.text_muted)
        .wrap()
        .grow(1.0)
        .min_w(0.0),
    );
    if !inspection.tree && ranges.len() > 1 {
        for (name, delta, disabled) in [
            ("Previous", -1_isize, page == 0),
            ("Next", 1, page + 1 == ranges.len()),
        ] {
            let scroll = scroll_key.clone();
            footer = footer.child(
                Button::new(name)
                    .disabled(disabled)
                    .on_press_tx(move |s: &mut S, tx| {
                        if let Some(v) = &mut get(s).inspection {
                            v.page = v.page.saturating_add_signed(delta);
                        }
                        tx.scroll_to_y(scroll.clone(), 0.0);
                    })
                    .build(k)
                    .key(format!("{root_key}:{}", name.to_lowercase())),
            );
        }
    }
    col()
        .key(&root_key)
        .pin(None, Some(12.0), Some(12.0), Some(12.0))
        .w((state.viewport[0] - 24.0).clamp(1.0, 560.0))
        .min_h(0.0)
        .padding(14.0)
        .gap(10.0)
        .fill(k.c.bg_base)
        .border(1.0, k.c.border_strong)
        .radius(10.0)
        .clip_rounded(10.0)
        .role("region")
        .label(format!("Inspect {}", node.title))
        .on_drag(|_: &mut S, _, _| {})
        .on_wheel(|_: &mut S, _, _| true)
        .on_magnify(|_: &mut S, _, _| true)
        .on_key(move |s: &mut S, e, tx| {
            if e.key == Key::Escape {
                get(s).inspection = None;
                tx.focus_control(escape_return.clone());
                return true;
            }
            if matches!(&e.key, Key::Text(t) if ["+", "=", "-", "0", "1"].contains(&t.as_str())) {
                return true;
            }
            matches!(
                e.key,
                Key::Left
                    | Key::Right
                    | Key::Up
                    | Key::Down
                    | Key::Home
                    | Key::End
                    | Key::PageUp
                    | Key::PageDown
            )
        })
        .child(header)
        .child(controls)
        .child(
            col()
                .grow(1.0)
                .min_h(0.0)
                .border(1.0, k.c.border_base)
                .radius(6.0)
                .clip_rounded(6.0)
                .child(body),
        )
        .child(footer)
}

#[cfg(test)]
mod tests {
    #[test]
    fn paging_preserves_every_unicode_character() {
        let source = "日本語🦀".repeat(8000);
        let ranges = super::pages(&source);
        assert!(ranges.len() > 1);
        assert_eq!(
            ranges
                .iter()
                .map(|r| &source[r.clone()])
                .collect::<String>(),
            source
        );
        assert!(ranges.iter().all(|r| r.len() <= 16_000));
    }
}
