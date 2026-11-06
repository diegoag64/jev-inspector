use super::*;

/// Keep the pane's geometry stable. Replace its contents as one surface so long JSON,
/// cards and diagrams never interpolate their individual text/layout into each other.
pub fn detail(s: &State, body: El) -> El {
    let phase = match &s.detail {
        None if s.detail_error.is_some() || s.connection_error.is_some() => "unavailable",
        None => "loading",
        Some(d) if d.summary.outcome == Outcome::Pending => "pending",
        Some(_) => "ready",
    };
    col()
        .key("detail-stage")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .clip_soft(0.0)
        .child(
            col()
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .key(format!(
                    "detail-content-{}-{}-{phase}",
                    s.selected.unwrap_or(0),
                    s.tab
                ))
                .child(body)
                .instant_changes()
                .transition(
                    rune::Transition::asymmetric(
                        rune::Transition::offset(0.0, 6.0).combined(rune::Transition::opacity()),
                        rune::Transition::none(),
                    )
                    .springs(rune::SpringPair::new(
                        rune::Spring::new(0.22, 0.0),
                        rune::Spring::NONE,
                    )),
                ),
        )
}

pub fn metric(key: &str, value: String, size: f32, color: Color, k: &Kit) -> El {
    rune_kit::RollingText::new(format!("inspector-metric-{key}"), value.clone())
        .font(rune::TextStyle { size, ..k.ty.ui })
        .color(color)
        .spring(rune::Spring::new(0.28, 0.0))
        .build(k)
        .role("text")
        .label(value)
}
