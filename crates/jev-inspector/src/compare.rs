use super::*;
use rune_kit::diff::{DiffMode, DiffState, DiffView, FileDiff};

pub struct Comparison {
    pub baseline: Option<Exchange>,
    pub diff: DiffState,
    inputs: Vec<(&'static str, String, String)>,
    unavailable: Vec<String>,
    limited: bool,
    outside_preview: Vec<String>,
}
impl Default for Comparison {
    fn default() -> Self {
        let mut diff = DiffState::default();
        diff.set_mode(DiffMode::Unified);
        Self {
            baseline: None,
            diff,
            inputs: vec![],
            unavailable: vec![],
            limited: false,
            outside_preview: vec![],
        }
    }
}
fn body(c: &jev_protocol::Capture) -> (String, bool) {
    let Some(value) = &c.json else {
        return ("No captured JSON\n".into(), false);
    };
    let full = serde_json::to_string_pretty(value).unwrap();
    let mut bounded = String::new();
    for line in full.split_inclusive('\n').take(800) {
        let remaining = 16_000usize.saturating_sub(bounded.len());
        let mut end = line.len().min(remaining);
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        bounded.push_str(&line[..end]);
        if end < line.len() {
            break;
        }
    }
    let limited = full != bounded;
    (bounded, limited)
}
impl Comparison {
    pub fn update(&mut self, current: &Exchange) {
        let Some(base) = &self.baseline else {
            return;
        };
        let mut inputs = vec![];
        self.limited = false;
        self.unavailable.clear();
        self.outside_preview.clear();
        for (name, a, b) in [
            ("request.json", &base.request, &current.request),
            ("response.json", &base.response, &current.response),
        ] {
            if a.json.is_none() || b.json.is_none() {
                let side = match (a.json.is_some(), b.json.is_some()) {
                    (false, false) => "both exchanges",
                    (false, true) => "the baseline",
                    _ => "the selected exchange",
                };
                self.unavailable.push(format!(
                    "{name}: no captured JSON in {side}. Equality is unknown."
                ));
                continue;
            }
            let equal = a.json == b.json;
            let (a, limit_a) = body(a);
            let (b, limit_b) = body(b);
            self.limited |= limit_a || limit_b;
            if !equal && a == b {
                self.outside_preview.push(name.to_owned());
                continue;
            }
            inputs.push((name, a, b));
        }
        if inputs != self.inputs {
            self.diff.set_files(
                inputs
                    .iter()
                    .map(|(name, a, b)| FileDiff::from_texts(*name, a, b))
                    .collect(),
            );
            // Keep equal bodies available without spending a row on an empty diff.
            for index in 0..self.diff.files().len() {
                if self.diff.files()[index].hunks.is_empty() {
                    self.diff.set_collapsed(index, true);
                }
            }
            self.inputs = inputs;
        }
    }
}
fn capture_coverage(exchange: &Exchange) -> String {
    let mut issues = vec![];
    for (name, capture) in [
        ("request", &exchange.request),
        ("response", &exchange.response),
    ] {
        if capture.json.is_none() {
            issues.push(format!("{name} JSON unavailable"));
        } else if !capture.complete {
            issues.push(format!("{name} incomplete"));
        }
    }
    if issues.is_empty() {
        "complete".into()
    } else {
        issues.join(", ")
    }
}

pub fn view(s: &State, current: &Exchange, k: &Kit, cx: &Cx<State>) -> El {
    let cmp = &s.workspace.comparison;
    let mut panel = col()
        .grow(1.0)
        .min_h(0.0)
        .gap(if s.workspace.window_height < 800.0 {
            8.0
        } else {
            12.0
        });
    let Some(base) = &cmp.baseline else {
        return panel.child(label("Compare requests", 20.0, k.c.text_base))
            .child(label("Set this request as the baseline, then select another request to compare their captured JSON.", 13.0, k.c.text_muted).wrap())
            .child(row().child(ui::button("Set baseline", "set-baseline", true, k, |s| {
                if let Some(detail) = &s.detail { s.workspace.comparison.baseline = Some(detail.clone()); s.workspace.comparison.update(detail); }
            })));
    };
    panel = panel.child(
        row()
            .gap(8.0)
            .flex_wrap()
            .center_y()
            .child(
                label(
                    format!(
                        "Baseline #{:03} → Current #{:03}",
                        base.summary.id, current.summary.id
                    ),
                    13.0,
                    k.c.text_accent,
                )
                .grow(1.0),
            )
            .child(ui::button(
                if cmp.diff.mode() == DiffMode::Unified {
                    "View: Unified"
                } else {
                    "View: Split"
                },
                "diff-mode",
                false,
                k,
                |s| {
                    let diff = &mut s.workspace.comparison.diff;
                    diff.set_mode(if diff.mode() == DiffMode::Unified {
                        DiffMode::Split
                    } else {
                        DiffMode::Unified
                    });
                },
            ))
            .child(
                Button::new(if s.workspace.detail_expanded {
                    "Restore"
                } else {
                    "Expand"
                })
                .icon(if s.workspace.detail_expanded {
                    &icons::RESTORE
                } else {
                    &icons::EXPAND
                })
                .variant(
                    if cmp.diff.mode() == DiffMode::Split
                        && !s.workspace.detail_expanded
                        && s.workspace.window_width - s.workspace.list_width < 750.0
                    {
                        ButtonVariant::Contrast
                    } else {
                        ButtonVariant::Ghost
                    },
                )
                .help("Give the comparison the full workspace width")
                .on_press(State::toggle_expanded)
                .build(k)
                .key("expand-comparison"),
            )
            .child(
                ui::action(&rune_kit::glyphs::CLOSE, "Clear baseline", false)
                    .on_press(|s: &mut State| s.workspace.comparison = Comparison::default())
                    .build(k)
                    .key("clear-baseline"),
            ),
    );
    let baseline_coverage = capture_coverage(base);
    let current_coverage = capture_coverage(current);
    if baseline_coverage != "complete" || current_coverage != "complete" {
        panel = panel.child(
            col()
                .gap(4.0)
                .key("comparison-capture-context")
                .child(
                    label(
                        format!("Baseline: {baseline_coverage} · Current: {current_coverage}"),
                        11.0,
                        k.c.warning.fg,
                    )
                    .wrap()
                    .role("status"),
                )
                .child(
                    label(
                        "Comparison applies to captured JSON. Incomplete bodies may omit data.",
                        11.0,
                        k.c.text_muted,
                    )
                    .wrap(),
                ),
        );
    }
    if cmp.diff.mode() == DiffMode::Split
        && !s.workspace.detail_expanded
        && s.workspace.window_width - s.workspace.list_width < 750.0
    {
        panel = panel.child(
            label(
                "For wider lines, choose Unified or Expand.",
                11.0,
                k.c.text_muted,
            )
            .wrap(),
        );
    }
    if let (Some(a), Some(b)) = (base.summary.duration_ms, current.summary.duration_ms) {
        panel = panel.child(label(
            format!(
                "Elapsed {} → {}  ({:+.1} ms)",
                ui::duration(Some(a)),
                ui::duration(Some(b)),
                b - a
            ),
            12.0,
            k.c.text_muted,
        ));
    }
    if cmp.limited {
        panel = panel.child(label("Diff preview limited to 16,000 bytes / 800 lines per body. Equality checks use complete captured JSON.", 11.0, k.c.warning.fg).wrap());
    }
    for message in &cmp.unavailable {
        panel = panel.child(label(message, 12.0, k.c.warning.fg).wrap().role("status"));
    }
    for name in &cmp.outside_preview {
        let request = name == "request.json";
        panel = panel.child(
            ui::surface(col(), ui::CARD_RADIUS, k.c.bg_deep)
                .gap(8.0)
                .padding(12.0)
                .key(format!("comparison-outside-preview-{name}"))
                .child(
                    row()
                        .center_y()
                        .gap(8.0)
                        .child(label(name, 13.0, k.c.text_base).grow(1.0))
                        .child(ui::badge("Different beyond preview", k.c.warning.fg, k)),
                )
                .child(
                    label(
                        "The captured values differ beyond this diff preview.",
                        12.0,
                        k.c.text_muted,
                    )
                    .wrap()
                    .role("status"),
                )
                .child(
                    row()
                        .gap(8.0)
                        .flex_wrap()
                        .child(
                            Button::new("Open current")
                                .variant(ButtonVariant::Ghost)
                                .on_press_tx(move |s: &mut State, tx| {
                                    s.select_tab(if request { 1 } else { 2 });
                                    tx.blur();
                                })
                                .build(k)
                                .key(format!("compare-open-{name}")),
                        )
                        .child(
                            Button::new("Copy baseline")
                                .icon(&rune_kit::glyphs::COPY)
                                .variant(ButtonVariant::Ghost)
                                .on_press_tx(move |s: &mut State, tx| {
                                    if let Some(base) = &s.workspace.comparison.baseline {
                                        let capture = if request {
                                            &base.request
                                        } else {
                                            &base.response
                                        };
                                        if let Some(value) = &capture.json {
                                            tx.copy(serde_json::to_string_pretty(value).unwrap());
                                            s.workspace.notice = Some((
                                                format!(
                                                    "Baseline #{} {} JSON copied",
                                                    base.summary.id,
                                                    if request { "request" } else { "response" }
                                                ),
                                                false,
                                            ));
                                        }
                                    }
                                })
                                .build(k)
                                .key(format!("compare-copy-{name}")),
                        ),
                ),
        );
    }
    if cmp.inputs.is_empty() && !cmp.outside_preview.is_empty() {
        return panel;
    }
    if !cmp.outside_preview.is_empty() && cmp.diff.files().iter().all(|file| file.hunks.is_empty())
    {
        for (name, _, _) in &cmp.inputs {
            panel = panel.child(
                row()
                    .center_y()
                    .padding(12.0)
                    .child(label(*name, 13.0, k.c.text_base).grow(1.0))
                    .child(ui::badge("Unchanged", k.c.text_muted, k)),
            );
        }
        return panel;
    }
    if cmp.inputs.is_empty() {
        return panel.child(label("No captured bodies available to compare", 16.0, k.c.text_base))
            .child(label("Choose exchanges with captured JSON. Missing bodies cannot establish whether requests or responses match.", 13.0, k.c.text_muted).wrap());
    }
    let header_kit = k.clone();
    panel.child(request_scroll::area(
        cx,
        k,
        "exchange-diff/scroll",
        "exchange-diff/scroll/rows",
        DiffView::new(
            |s: &State| &s.workspace.comparison.diff,
            |s| &mut s.workspace.comparison.diff,
        )
        .key("exchange-diff")
        .header_actions(move |s: &State, index| {
            if s.workspace.comparison.diff.files()[index].hunks.is_empty() {
                ui::badge("Unchanged", header_kit.c.text_muted, &header_kit)
            } else {
                el()
            }
        })
        .build(s, k)
        .grow(1.0)
        .min_h(0.0)
        .on_key(|s: &mut State, ev, tx| s.workspace.comparison.diff.key(ev, tx)),
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn different_tails_never_become_unchanged_after_either_preview_limit() {
        for prefix in [
            serde_json::json!("🦀".repeat(6000)),
            serde_json::json!((0..1000).collect::<Vec<_>>()),
        ] {
            let mut a = demo::exchange(1);
            let mut b = demo::exchange(2);
            a.response.json = Some(serde_json::json!({"a":prefix,"z":1}));
            b.response.json = Some(serde_json::json!({"a":prefix,"z":2}));
            let mut cmp = Comparison {
                baseline: Some(a.clone()),
                ..Default::default()
            };
            cmp.update(&b);
            assert_eq!(cmp.outside_preview.len(), 1);
            assert!(cmp.outside_preview[0].contains("response.json"));
            assert_eq!(
                cmp.diff.files().len(),
                1,
                "Only the actually equal request has an unchanged diff"
            );
            cmp.update(&a);
            assert!(cmp.outside_preview.is_empty());
            assert_eq!(cmp.diff.files().len(), 2);
            b.response.json = None;
            cmp.update(&b);
            assert!(cmp.outside_preview.is_empty());
            assert_eq!(cmp.unavailable.len(), 1);
        }
    }
    #[test]
    fn absent_bodies_are_unavailable_instead_of_equal() {
        for missing_baseline in [true, false] {
            let mut base = demo::exchange(1);
            let mut current = demo::exchange(2);
            if missing_baseline {
                base.request.json = None;
            } else {
                current.request.json = None;
            }
            let mut cmp = Comparison {
                baseline: Some(base.clone()),
                ..Default::default()
            };
            cmp.update(&current);
            assert_eq!(cmp.diff.files().len(), 1);
            assert_eq!(cmp.unavailable.len(), 1);
            base.request.json = None;
            base.response.json = None;
            current.request.json = None;
            current.response.json = None;
            cmp.baseline = Some(base);
            cmp.update(&current);
            assert!(cmp.diff.files().is_empty());
            assert_eq!(cmp.unavailable.len(), 2);
            assert!(cmp.unavailable.iter().all(|s| s.contains("both exchanges")));
        }
    }
    #[test]
    fn compares_requests_and_responses_and_bounds_unicode_input() {
        let mut c = Comparison {
            baseline: Some(demo::exchange(12)),
            ..Default::default()
        };
        c.update(&demo::exchange(7));
        assert_eq!(c.diff.files().len(), 2);
        assert!(c.diff.collapsed(0));
        assert!(!c.diff.collapsed(1));
        assert!(!c.diff.files()[1].hunks.is_empty());
        let mut large = demo::exchange(1);
        large.request.json = Some(serde_json::json!({"text": "🦀".repeat(9000)}));
        let (text, limited) = body(&large.request);
        assert!(limited);
        assert!(text.len() <= 16000);
    }
}
