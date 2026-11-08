use crate::{State, Summary, label, ui};
use jev_protocol::{Outcome, TokenUsage};
use rune::{El, TextEdit, col, el, row};
use rune_kit::{Button, ButtonVariant, Dialog, Kit, TextInput};
use serde::{Deserialize, Serialize};

/// USD per million tokens. Applied only to POST /v1/systemone.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rates {
    pub input: f64,
    pub output: f64,
}
impl Default for Rates {
    fn default() -> Self {
        // https://docs.typesafe.ai/models — verified 2026-11-07.
        Self {
            input: 0.042,
            output: 0.0,
        }
    }
}
impl Rates {
    fn valid(self) -> bool {
        [self.input, self.output]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1_000_000.0).contains(v))
    }
}

#[derive(Default)]
pub struct CostState {
    pub enabled: bool,
    pub settings: bool,
    pub rates: Rates,
    input: TextEdit,
    output: TextEdit,
}
impl CostState {
    pub fn load(&mut self, prefs: &rune::prefs::Prefs) {
        self.rates = prefs
            .get::<Rates>("cost_rates")
            .filter(|r| r.valid())
            .unwrap_or_default();
        self.enabled = prefs.get("cost_mode").unwrap_or(false);
    }
    fn draft(&self) -> Option<Rates> {
        let rates = Rates {
            input: self.input.text.trim().parse().ok()?,
            output: self.output.text.trim().parse().ok()?,
        };
        rates.valid().then_some(rates)
    }
    fn edit(&mut self) {
        self.input = TextEdit::new(self.rates.input.to_string());
        self.output = TextEdit::new(self.rates.output.to_string());
        self.settings = true;
    }
}

pub fn toggle(s: &mut State) {
    s.cost.enabled = !s.cost.enabled;
    // The two modes have different columns; discard a now-invisible sort.
    s.table.clear_sort();
    s.filter_rows();
    if let Err(e) = s.workspace.prefs.set("cost_mode", &s.cost.enabled) {
        s.workspace.notice = Some((e.to_string(), true));
    }
}

pub fn usage(s: &State, row: &Summary) -> Option<TokenUsage> {
    row.usage.or_else(|| {
        // Older sessions/relays have no usage in their summary. Only use a
        // complete captured response, never guess token counts from byte size.
        s.workspace
            .cache
            .get(&row.id)
            .and_then(|e| e.response.token_usage())
    })
}

#[derive(Debug, PartialEq)]
pub enum Estimate {
    Known(f64),
    Pending,
    Missing,
    Unsupported,
}
impl Estimate {
    pub fn amount(&self) -> Option<f64> {
        match self {
            Self::Known(v) => Some(*v),
            _ => None,
        }
    }
    pub fn display(&self) -> String {
        match self {
            Self::Known(v) => money(*v),
            Self::Pending => "Pending".into(),
            Self::Missing => "No usage".into(),
            Self::Unsupported => "N/A".into(),
        }
    }
}

pub fn estimate(s: &State, row: &Summary) -> Estimate {
    if row.method != "POST" || row.path.split('?').next() != Some("/v1/systemone") {
        return Estimate::Unsupported;
    }
    if row.outcome == Outcome::Pending {
        return Estimate::Pending;
    }
    let Some(usage) = usage(s, row) else {
        return Estimate::Missing;
    };
    let Some(input) = usage.input_tokens else {
        return Estimate::Missing;
    };
    let output = match usage.output_tokens {
        Some(v) => v,
        None if s.cost.rates.output == 0.0 => 0,
        None => return Estimate::Missing,
    };
    Estimate::Known(
        (input as f64 * s.cost.rates.input + output as f64 * s.cost.rates.output) / 1_000_000.0,
    )
}

pub fn money(value: f64) -> String {
    if value == 0.0 {
        "$0.00".into()
    } else if value < 0.000001 {
        "<$0.000001".into()
    } else if value < 1.0 {
        format!("${value:.6}")
    } else {
        format!("${value:.4}")
    }
}

#[derive(Default, Debug)]
pub struct Totals {
    pub usd: f64,
    pub priced: usize,
    pub pending: usize,
    pub missing: usize,
    pub unsupported: usize,
    pub input: u128,
}
pub fn totals(s: &State) -> Totals {
    let mut total = Totals::default();
    for row in &s.visible {
        match estimate(s, row) {
            Estimate::Known(v) => {
                total.usd += v;
                total.priced += 1;
            }
            Estimate::Pending => total.pending += 1,
            Estimate::Missing => total.missing += 1,
            Estimate::Unsupported => total.unsupported += 1,
        }
        if !matches!(estimate(s, row), Estimate::Unsupported | Estimate::Pending) {
            total.input += usage(s, row).and_then(|u| u.input_tokens).unwrap_or(0) as u128;
        }
    }
    total
}

pub fn summary(s: &State, k: &Kit) -> El {
    let total = totals(s);
    let partial = total.pending + total.missing + total.unsupported > 0;
    let headline = if total.priced == 0 && !s.visible.is_empty() {
        "Unavailable".into()
    } else {
        money(total.usd)
    };
    let mut coverage = format!("{} / {} estimated", total.priced, s.visible.len());
    for (count, text) in [
        (total.missing, "no usage"),
        (total.pending, "pending"),
        (total.unsupported, "unsupported"),
    ] {
        if count > 0 {
            coverage.push_str(&format!(" · {count} {text}"));
        }
    }
    col()
        .key("cost-summary")
        .px(14.0)
        .py(10.0)
        .gap(6.0)
        .fill(k.c.layer_02)
        .child(
            row()
                .center_y()
                .gap(10.0)
                .child(
                    col()
                        .grow(1.0)
                        .gap(3.0)
                        .child(label(
                            if partial {
                                "PARTIAL ESTIMATE · USD"
                            } else {
                                "ESTIMATED COST · USD"
                            },
                            10.0,
                            k.c.text_muted,
                        ))
                        .child(ui::mono(headline, 20.0, k.c.text_base).key("cost-total")),
                )
                .child(
                    Button::new("Pricing")
                        .variant(ButtonVariant::Ghost)
                        .help("Set input and output rates in USD per million tokens")
                        .on_press(|s: &mut State| s.cost.edit())
                        .build(k)
                        .key("cost-pricing"),
                ),
        )
        .child(
            label(coverage, 11.0, k.c.text_muted)
                .wrap()
                .key("cost-coverage"),
        )
        .child(
            label(
                format!(
                    "{} reported input tokens · filtered Traffic{}",
                    total.input,
                    if s.demo && s.workspace.session_name.is_none() {
                        " · synthetic usage"
                    } else {
                        ""
                    }
                ),
                11.0,
                k.c.text_muted,
            )
            .wrap(),
        )
}

pub fn row_help(s: &State, r: &Summary) -> String {
    match estimate(s, r) {
        Estimate::Known(value) => {
            let u = usage(s, r).unwrap();
            format!("Estimated USD {value:.9} · {} input / {} output tokens · rates ${}/M input + ${}/M output. Uses reported usage; not a bill.",
                u.input_tokens.unwrap(), u.output_tokens.map_or("unreported".into(), |v| v.to_string()), s.cost.rates.input, s.cost.rates.output)
        }
        Estimate::Pending => "Awaiting completion and reported token usage; excluded from the estimate.".into(),
        Estimate::Missing => "Required token usage is unavailable. This request is excluded, not assumed free. Older relays may require opening the request or rebuilding the relay.".into(),
        Estimate::Unsupported => "Only POST /v1/systemone is covered by this pricing profile.".into(),
    }
}

pub fn dialog(s: &State, k: &Kit, layers: &mut Vec<El>) {
    let invalid = s.cost.draft().is_none();
    let body = col().padding(20.0).gap(14.0)
        .child(label("USD per 1 million tokens", 14.0, k.c.text_base))
        .child(label("Applies to all visible POST /v1/systemone requests, including saved sessions. Estimates use reported response usage; unknown usage is excluded.", 12.0, k.c.text_muted).wrap())
        .child(row().gap(12.0)
            .child(col().grow(1.0).min_w(0.0).gap(6.0)
                .child(label("Input rate", 12.0, k.c.text_muted))
                .child(TextInput::new(&s.cost.input).width(None).label("Input USD per million tokens")
                    .on_edit(|s: &mut State| &mut s.cost.input).invalid(invalid).build(k).key("cost-input-rate")))
            .child(col().grow(1.0).min_w(0.0).gap(6.0)
                .child(label("Output rate", 12.0, k.c.text_muted))
                .child(TextInput::new(&s.cost.output).width(None).label("Output USD per million tokens")
                    .on_edit(|s: &mut State| &mut s.cost.output).invalid(invalid).build(k).key("cost-output-rate"))))
        .child(label(if invalid { "Enter a number from 0 to 1,000,000 for each rate." } else { "Default: Jev $0.042 input / $0 output. Verified Nov 7, 2026." }, 12.0,
            if invalid { k.c.danger.fg } else { k.c.text_muted }).wrap())
        .child(label("Source: docs.typesafe.ai/models. Custom rates are saved locally for live use. No request is sent when you change pricing.", 11.0, k.c.text_muted).wrap())
        .child(row().gap(8.0)
            .child(Button::new("Reset to Jev").variant(ButtonVariant::Ghost).on_press(|s: &mut State| {
                s.cost.input = TextEdit::new(Rates::default().input.to_string());
                s.cost.output = TextEdit::new("0");
            }).build(k).key("cost-reset"))
            .child(el().grow(1.0))
            .child(Button::new("Cancel").variant(ButtonVariant::Ghost).on_press(|s: &mut State| s.cost.settings = false).build(k).key("cost-cancel"))
            .child(Button::new("Apply rates").variant(ButtonVariant::Contrast).disabled(invalid)
                .on_press(|s: &mut State| {
                    if let Some(rates) = s.cost.draft() {
                        s.cost.rates = rates;
                        s.cost.settings = false;
                        if let Err(e) = s.workspace.prefs.set("cost_rates", &rates) {
                            s.workspace.notice = Some((e.to_string(), true));
                        }
                        s.filter_rows();
                    }
                }).build(k).key("cost-apply")));
    Dialog::new("cost-settings", "Cost estimation")
        .description("Configure the Jev pricing profile")
        .open(s.cost.settings)
        .width(520.0)
        .fit()
        .on_close(|s: &mut State| s.cost.settings = false)
        .child(body)
        .build(k, layers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        showcase,
        showcase_tests::{click, runtime},
    };

    #[test]
    fn estimates_use_reported_usage_and_handle_unknowns_without_claiming_zero() {
        let mut s = State::default();
        let mut row = Summary {
            method: "POST".into(),
            path: "/v1/systemone?trace=1".into(),
            outcome: Outcome::Complete,
            usage: Some(TokenUsage {
                input_tokens: Some(1_000_000),
                output_tokens: None,
            }),
            ..Summary::default()
        };
        assert_eq!(estimate(&s, &row), Estimate::Known(0.042));
        s.cost.rates.output = 0.1;
        assert_eq!(estimate(&s, &row), Estimate::Missing);
        row.usage.as_mut().unwrap().output_tokens = Some(500_000);
        assert!((estimate(&s, &row).amount().unwrap() - 0.092).abs() < 1e-12);
        row.status = Some(500);
        assert!(
            estimate(&s, &row).amount().is_some(),
            "reported usage still counts on errors"
        );
        row.usage = None;
        assert_eq!(estimate(&s, &row), Estimate::Missing);
        row.outcome = Outcome::Pending;
        assert_eq!(estimate(&s, &row), Estimate::Pending);
        row.path = "/v1/models".into();
        assert_eq!(estimate(&s, &row), Estimate::Unsupported);
        assert_eq!(money(0.000000042), "<$0.000001");
        assert_ne!(money(0.000012), "$0.00");
    }

    #[test]
    fn totals_follow_filters_clear_restore_pause_and_saved_legacy_captures() {
        let mut s = State::default();
        showcase::populate(&mut s);
        let all = totals(&s);
        assert_eq!((all.priced, all.missing), (9, 1));
        assert!(all.usd > 0.0);
        s.table.set_sort("cost", rune_kit::SortDir::Descending);
        s.filter_rows();
        assert_eq!(s.visible[0].id, 7, "inventory has the highest cost");
        assert_eq!(
            s.visible.last().unwrap().id,
            10,
            "unknown cost remains separate"
        );
        let snapshot: jev_protocol::Snapshot =
            serde_json::from_slice(&serde_json::to_vec(&s.snapshot).unwrap()).unwrap();
        s.snapshot = snapshot;
        assert_eq!(
            totals(&s).usd,
            all.usd,
            "usage survives session serialization"
        );
        s.filter = TextEdit::new("id:7");
        s.filter_rows();
        assert_eq!(totals(&s).priced, 1);
        let expected =
            s.visible[0].usage.unwrap().input_tokens.unwrap() as f64 * 0.042 / 1_000_000.0;
        assert_eq!(totals(&s).usd, expected);
        s.workspace.session_name = Some("legacy-session.json".into());
        for r in &mut s.snapshot.requests {
            r.usage = None;
        }
        s.filter_rows();
        assert_eq!(
            totals(&s).usd,
            expected,
            "old session capture supplies usage"
        );
        s.workspace.cache.clear();
        assert_eq!(
            totals(&s).missing,
            1,
            "summary-only legacy sessions stay unknown"
        );
        s.clear_view();
        assert_eq!(totals(&s).priced + totals(&s).missing, 0);
        s.workspace.clear_before = 0;
        s.filter_rows();
        assert_eq!(totals(&s).missing, 1);
        s.workspace.pending = Some(jev_protocol::Snapshot::default());
        s.workspace.paused = true;
        assert_eq!(
            totals(&s).missing,
            1,
            "pending live snapshot does not change frozen totals"
        );
    }

    #[test]
    fn rates_validate_and_round_trip_preferences() {
        let mut cost = CostState::default();
        cost.edit();
        for value in ["", "NaN", "inf", "-1", "1000001", "abc"] {
            cost.input = TextEdit::new(value);
            assert!(cost.draft().is_none(), "{value}");
        }
        cost.input = TextEdit::new("0.5");
        cost.output = TextEdit::new("1.25");
        let rates = cost.draft().unwrap();
        let mut prefs = rune::prefs::Prefs::memory();
        prefs.set("cost_rates", &rates).unwrap();
        prefs.set("cost_mode", &true).unwrap();
        cost.load(&prefs);
        assert_eq!(cost.rates, rates);
        assert!(cost.enabled);
    }

    #[test]
    fn cost_controls_fit_and_pricing_dialog_contains_keyboard_actions() {
        for dark in [true, false] {
            let mut rt = runtime();
            rt.set_viewport((1100.0, 700.0), 1.5);
            rt.update(|s, _| {
                s.dark = dark;
                s.tab = 0;
            });
            rt.frame(0.3);
            click(&mut rt, "cost-mode");
            assert!(rt.state.cost.enabled);
            let (rect, _) = rt.presented("cost-summary").unwrap();
            assert!(rect[2] >= 400.0 && rect[3] < 125.0);
            click(&mut rt, "cost-pricing");
            assert!(rt.state.cost.settings);
            let previous = rt.state.selected;
            rt.key(rune::KeyEvent {
                key: rune::Key::Command('l'),
                mods: rune::Mods {
                    cmd: true,
                    ..Default::default()
                },
            });
            rt.frame(0.1);
            assert_eq!(
                rt.state.workspace.clear_before, 0,
                "modal blocks Clear view shortcut"
            );
            assert_eq!(rt.state.selected, previous);
            rt.update(|s, _| {
                s.cost.input = TextEdit::new("2");
                s.cost.output = TextEdit::new("3");
            });
            rt.frame(0.1);
            click(&mut rt, "cost-apply");
            assert_eq!(
                rt.state.cost.rates,
                Rates {
                    input: 2.0,
                    output: 3.0
                }
            );
            assert!(!rt.state.cost.settings);
            click(&mut rt, "cost-pricing");
            rt.update(|s, _| s.cost.input = TextEdit::new("-4"));
            rt.frame(0.1);
            click(&mut rt, "cost-apply");
            assert!(rt.state.cost.settings, "invalid rates cannot be applied");
            assert_eq!(rt.state.cost.rates.input, 2.0);
            click(&mut rt, "cost-cancel");
            assert_eq!(rt.state.cost.rates.input, 2.0);
            click(&mut rt, "cost-mode");
            assert!(!rt.state.cost.enabled);
        }
    }
}
