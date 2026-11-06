use super::*;

// Rune's Table owns its viewport and does not expose a scrollbar builder. Keep
// its virtual rows, selection and keyboard handling; attach a separate gutter
// that reads the viewport's actual geometry and scrolls that same viewport.
pub fn view(cx: &Cx<State>, k: &Kit, header: f32) -> El {
    vertical(
        cx,
        k,
        header,
        "requests/viewport",
        "requests/viewport/rows",
        "request-scrollbar",
        "request-scroll-thumb",
    )
}

pub fn vertical(
    cx: &Cx<State>,
    k: &Kit,
    header: f32,
    viewport_key: &'static str,
    rows_key: &'static str,
    bar_key: &'static str,
    thumb_key: &'static str,
) -> El {
    let viewport = cx.window_layout(viewport_key);
    let rows = cx.window_layout(rows_key);
    let (Some(v), Some(r)) = (viewport, rows) else {
        return el();
    };
    let max = (r[3] - v[3]).max(0.0);
    if max <= 0.5 || v[3] <= 0.0 {
        return el();
    }
    let track = (v[3] - 4.0).max(1.0);
    let height = (track * v[3] / r[3]).clamp(24.0_f32.min(track), track);
    let travel = track - height;
    let offset = (v[1] - r[1]).clamp(0.0, max);
    let horizontal = (v[0] - r[0]).max(0.0);
    let top = 2.0 + travel * offset / max;
    el().key(bar_key)
        .pin(None, Some(header), Some(0.0), Some(0.0))
        .w(16.0)
        .fill(k.c.layer_02)
        .a11y_hidden()
        .on_drag(move |_: &mut State, d, tx| {
            if d.phase == rune::Phase::Down {
                let next = if d.y < top {
                    offset - v[3]
                } else {
                    offset + v[3]
                };
                tx.scroll_to_instant(viewport_key, horizontal, next.clamp(0.0, max));
            }
        })
        .child(
            el().key(thumb_key)
                .absolute(4.0, top)
                .size(8.0, height)
                .radius(4.0)
                .fill(k.c.text_faint)
                .hover(k.c.text_muted)
                .on_drag(move |_: &mut State, d, tx| {
                    if d.cancelled() || travel <= 0.0 {
                        return;
                    }
                    if d.phase == rune::Phase::Down {
                        d.data = offset;
                    }
                    tx.scroll_to_instant(
                        viewport_key,
                        horizontal,
                        (d.data + d.dy * max / travel).clamp(0.0, max),
                    );
                }),
        )
}

/// Persistent gutters for inspector detail panes, including both axes of code.
pub fn area(
    cx: &Cx<State>,
    k: &Kit,
    viewport: &'static str,
    content: &'static str,
    body: El,
    both: bool,
) -> El {
    col()
        .grow(1.0)
        .min_h(0.0)
        .min_w(0.0)
        .pr(16.0)
        .pb(if both { 16.0 } else { 0.0 })
        .child(body)
        .child(vertical(
            cx,
            k,
            0.0,
            viewport,
            content,
            "detail-vertical-scrollbar",
            "detail-vertical-thumb",
        ))
        .child(if both {
            horizontal(cx, k, viewport, content)
        } else {
            el()
        })
}

fn horizontal(cx: &Cx<State>, k: &Kit, viewport: &'static str, content: &'static str) -> El {
    let (Some(v), Some(r)) = (cx.window_layout(viewport), cx.window_layout(content)) else {
        return el();
    };
    let max = (r[2] - v[2]).max(0.0);
    if max <= 0.5 || v[2] <= 0.0 {
        return el();
    }
    let track = (v[2] - 4.0).max(1.0);
    let width = (track * v[2] / r[2]).clamp(24.0_f32.min(track), track);
    let travel = track - width;
    let offset = (v[0] - r[0]).clamp(0.0, max);
    let vertical = (v[1] - r[1]).max(0.0);
    let left = 2.0 + travel * offset / max;
    el().key("detail-horizontal-scrollbar")
        .pin(Some(0.0), None, Some(16.0), Some(0.0))
        .h(16.0)
        .fill(k.c.layer_02)
        .a11y_hidden()
        .on_drag(move |_: &mut State, d, tx| {
            if d.phase == rune::Phase::Down {
                let next = if d.x < left {
                    offset - v[2]
                } else {
                    offset + v[2]
                };
                tx.scroll_to_instant(viewport, next.clamp(0.0, max), vertical);
            }
        })
        .child(
            el().key("detail-horizontal-thumb")
                .absolute(left, 4.0)
                .size(width, 8.0)
                .radius(4.0)
                .fill(k.c.text_faint)
                .hover(k.c.text_muted)
                .on_drag(move |_: &mut State, d, tx| {
                    if d.cancelled() || travel <= 0.0 {
                        return;
                    }
                    if d.phase == rune::Phase::Down {
                        d.data = offset;
                    }
                    tx.scroll_to_instant(
                        viewport,
                        (d.data + d.dx * max / travel).clamp(0.0, max),
                        vertical,
                    );
                }),
        )
}
