use crate::theme::{alpha, label, percent};
use crate::{ContentState, ProbabilityBars};
use rune::{El, ScrollSpec, TextStyle, col, el, row, text};
use rune_kit::Kit;

/// Presentation data supplied by the app; no probabilities or decisions are inferred here.
#[derive(Clone, Debug, Default)]
pub struct ResultContent {
    pub probabilities: Vec<(String, f64)>,
    pub selected: Option<String>,
    pub headline: Option<String>,
    pub caption: Option<String>,
    pub details: Vec<String>,
    pub confidence: Option<f64>,
    pub likelihood: Option<f64>,
}
impl ResultContent {
    pub(crate) fn build<S: 'static>(
        &self,
        key: &str,
        state: &ContentState,
        get: impl Fn(&mut S) -> &mut ContentState + Copy + 'static,
        k: &Kit,
        keyboard: bool,
    ) -> El {
        let mut body = col().w_full().gap(12.0);
        if !self.probabilities.is_empty() {
            body = body.child(
                ProbabilityBars::new(key, &self.probabilities, self.selected.as_deref())
                    .keyboard(keyboard)
                    .rows(state, get, k),
            );
        } else if let Some(selected) = &self.selected {
            body = body.child(label(format!("Selected: {selected}"), 14.0, k.c.text_base));
        }
        if let Some(headline) = &self.headline {
            body = body.child(
                text(headline)
                    .font(TextStyle::new(36.0, 500.0))
                    .color(k.c.text_base)
                    .pt(16.0),
            );
        }
        if let Some(caption) = &self.caption {
            body = body.child(label(caption, 12.0, k.c.text_muted).w_full().wrap());
        }
        if let Some(p) = self.likelihood {
            body = body
                .child(
                    el().h(7.0)
                        .w_full()
                        .radius(4.0)
                        .fill(alpha(k.c.text_base, 0.1))
                        .child(
                            el().absolute(0.0, 0.0)
                                .h(7.0)
                                .w_frac(p.clamp(0.0, 1.0) as f32)
                                .radius(4.0)
                                .fill(k.c.text_accent),
                        ),
                )
                .child(
                    row()
                        .justify_between()
                        .child(label("False", 12.0, k.c.text_muted))
                        .child(label("True", 12.0, k.c.text_muted)),
                );
        }
        for detail in &self.details {
            body = body.child(label(detail, 14.0, k.c.text_base).w_full().wrap_anywhere());
        }
        if let Some(c) = self.confidence {
            body = body
                .child(label(format!("Confidence {}", percent(c)), 12.0, k.c.text_muted).pt(8.0));
        }
        col()
            .grow(1.0)
            .min_h(0.0)
            .w_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(body)
    }
}
