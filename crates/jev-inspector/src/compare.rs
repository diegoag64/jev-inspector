use super::*;
use rune_kit::diff::{DiffMode, DiffState, DiffView, FileDiff};

#[derive(Default)]
pub struct Comparison {
    pub baseline: Option<Exchange>,
    pub diff: DiffState,
    inputs: Vec<(&'static str, String, String)>,
    unavailable: Vec<String>,
    limited: bool,
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
            let (a, limit_a) = body(a);
            let (b, limit_b) = body(b);
            self.limited |= limit_a || limit_b;
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
pub fn view(s: &State, current: &Exchange, k: &Kit) -> El {
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
        return panel.child(label("Spot what changed", 20.0, k.c.text_base))
            .child(label("Keep this exchange as a baseline, then select another request. Compare both input and output JSON with Rune’s diff viewer.", 13.0, k.c.text_muted).wrap())
            .child(ui::button("Use this exchange as baseline", "set-baseline", false, k, |s| {
                if let Some(detail) = &s.detail { s.workspace.comparison.baseline = Some(detail.clone()); s.workspace.comparison.update(detail); }
            }));
    };
    panel = panel.child(
        row()
            .gap(10.0)
            .center_y()
            .child(
                label(
                    format!("#{:03} → #{:03}", base.summary.id, current.summary.id),
                    13.0,
                    k.c.text_accent,
                )
                .grow(1.0),
            )
            .child(ui::button(
                if cmp.diff.mode() == DiffMode::Unified {
                    "Split"
                } else {
                    "Unified"
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
                .variant(ButtonVariant::Ghost)
                .help("Give the comparison the full workspace width")
                .on_press(State::toggle_expanded)
                .build(k)
                .key("expand-comparison"),
            )
            .child(ui::button(
                "Clear baseline",
                "clear-baseline",
                false,
                k,
                |s| s.workspace.comparison = Comparison::default(),
            )),
    );
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
        panel = panel.child(label("Comparison limited to 16,000 bytes / 800 lines per body. Export JSON for the full captured values.", 11.0, k.c.warning.fg).wrap());
    }
    for message in &cmp.unavailable {
        panel = panel.child(label(message, 12.0, k.c.warning.fg).wrap().role("status"));
    }
    if cmp.inputs.is_empty() {
        return panel.child(label("No captured bodies available to compare", 16.0, k.c.text_base))
            .child(label("Choose exchanges with captured JSON. Missing bodies cannot establish whether requests or responses match.", 13.0, k.c.text_muted).wrap());
    }
    let header_kit = k.clone();
    panel.child(
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
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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
