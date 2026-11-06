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
                tx.scroll_to_instant(viewport_key, 0.0, next.clamp(0.0, max));
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
                        0.0,
                        (d.data + d.dy * max / travel).clamp(0.0, max),
                    );
                }),
        )
}
