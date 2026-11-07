use super::*;

fn pending_context(s: &State) -> (&'static str, &'static str) {
    crate::capture_context::pending(s).timing()
}

fn elapsed(s: &State, r: &Summary) -> String {
    r.duration_ms
        .map(|v| ui::duration(Some(v)))
        .unwrap_or_else(|| {
            if r.outcome == Outcome::Pending {
                if s.workspace.session_name.is_some() {
                    "Not completed".into()
                } else {
                    pending_context(s).0.into()
                }
            } else {
                "Not recorded".into()
            }
        })
}

pub fn view(s: &State, detail: &Exchange, k: &Kit) -> El {
    let r = &detail.summary;
    let mut body = col().w_full().gap(18.0);
    if let Some(error) = detail.response.json.as_ref().and_then(|j| j.get("error")) {
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .or_else(|| error.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| error.to_string());
        body = body.child(
            col()
                .padding(14.0)
                .radius(8.0)
                .gap(6.0)
                .fill(k.alpha(k.c.danger.fg, 0.09))
                .border(1.0, k.alpha(k.c.danger.fg, 0.2))
                .child(label("Upstream error", 13.0, k.c.danger.fg))
                .child(
                    label(clip(&message, 1000), 13.0, k.c.text_base)
                        .wrap()
                        .selectable(true),
                ),
        );
    }
    body = body.child(
        row()
            .gap(12.0)
            .child(tile("Elapsed", elapsed(s, r), k))
            .child(tile("Sent", ui::bytes(r.request_bytes), k))
            .child(tile("Received", ui::bytes(r.response_bytes), k)),
    );
    body = body.child(ui::rule(k));
    if let Some(answers) = detail
        .response
        .json
        .as_ref()
        .and_then(|j| j.get("answers"))
        .and_then(|a| a.as_object())
    {
        body = body.child(
            row()
                .child(label("Answers", 15.0, k.c.text_base).grow(1.0))
                .child(label(
                    format!(
                        "{} {}",
                        answers.len(),
                        if answers.len() == 1 {
                            "question"
                        } else {
                            "questions"
                        }
                    ),
                    11.0,
                    k.c.text_faint,
                )),
        );
        for (name, answer) in answers.iter().take(32) {
            let kind = answer["type"].as_str().unwrap_or("unknown");
            let value = match kind {
                "choice" => &answer["choice"],
                "noul" => &answer["noul"],
                "score" => &answer["score"],
                "box" => &answer["box"],
                _ => answer,
            };
            let mut card = col()
                .gap(10.0)
                .padding(14.0)
                .radius(8.0)
                .fill(k.c.bg_deep)
                .border(1.0, k.c.border_base)
                .child(
                    row()
                        .center_y()
                        .gap(8.0)
                        .child(
                            label(name, 12.0, k.c.text_muted)
                                .grow(1.0)
                                .min_w(0.0)
                                .truncate_middle()
                                .help_when_truncated(
                                    name.chars()
                                        .collect::<Vec<_>>()
                                        .chunks(48)
                                        .map(|chunk| chunk.iter().collect::<String>())
                                        .collect::<Vec<_>>()
                                        .join("\n"),
                                ),
                        )
                        .child(ui::badge(kind.to_uppercase(), k.c.text_accent, k)),
                )
                .child(answer_value::view(kind, value, k));
            let mut probability_shown = 0;
            let mut probability_more = 0;
            if let Some(probabilities) = answer["probabilities"].as_object() {
                let mut values: Vec<_> = probabilities
                    .iter()
                    .filter_map(|(n, v)| v.as_f64().map(|v| (n, v)))
                    .collect();
                values.sort_by(|a, b| b.1.total_cmp(&a.1));
                let limit = s
                    .workspace
                    .probability_limits
                    .get(&(r.id, name.clone()))
                    .copied()
                    .unwrap_or(12);
                let choice = answer["choice"].as_str();
                probability_more = values
                    .iter()
                    .enumerate()
                    .filter(|(i, (n, _))| {
                        *i >= limit && *i < limit + 12 && Some(n.as_str()) != choice
                    })
                    .count();
                for (_, (name, probability)) in values
                    .into_iter()
                    .enumerate()
                    .filter(|(i, (n, _))| *i < limit || Some(n.as_str()) == choice)
                {
                    probability_shown += 1;
                    let p = probability.clamp(0.0, 1.0) as f32;
                    card = card.child(
                        row()
                            .gap(12.0)
                            .center_y()
                            .child(
                                label(name, 11.0, k.c.text_muted)
                                    .w(130.0)
                                    .truncate()
                                    .help(name),
                            )
                            .child(
                                row()
                                    .grow(1.0)
                                    .h(4.0)
                                    .radius(2.0)
                                    .clip_rounded(2.0)
                                    .fill(k.c.layer_02)
                                    .child(el().grow(p.max(0.0001)).h_full().fill(k.c.text_accent))
                                    .child(
                                        el().grow((1.0 - p).max(0.0001))
                                            .h_full()
                                            .fill(k.c.layer_02),
                                    ),
                            )
                            .child(
                                ui::mono(rune_diagram::percent(probability), 11.0, k.c.text_base)
                                    .w(55.0),
                            ),
                    );
                }
            }
            if let Some(probabilities) = answer["probabilities"].as_object() {
                let total = probabilities.values().filter(|v| v.is_number()).count();
                let limit = s
                    .workspace
                    .probability_limits
                    .get(&(r.id, name.clone()))
                    .copied()
                    .unwrap_or(12);
                if total > probability_shown {
                    let action = (r.id, name.clone());
                    card = card.child(label(format!("Showing {probability_shown} of {total} probabilities; returned choice is included."), 11.0, k.c.text_muted).wrap())
                        .child(Button::new(format!("Show {} more probabilities", probability_more))
                            .variant(ButtonVariant::Ghost)
                            .on_press_tx(move |s: &mut State, tx| { s.workspace.probability_limits.insert(action.clone(), limit + 12); tx.focus_control("detail-scroll"); })
                            .build(k).key(format!("probabilities-more-{name}")));
                }
            }
            if let Some(confidence) = answer["confidence"].as_f64() {
                card = card.child(label(
                    format!("Confidence {}", rune_diagram::percent(confidence)),
                    11.0,
                    k.c.text_faint,
                ));
            }
            body = body.child(card);
        }
        if answers.len() > 32 {
            body = body.child(
                label(
                    "Showing the first 32 answers. Open Response for the full captured JSON.",
                    11.0,
                    k.c.text_faint,
                )
                .wrap(),
            );
        }
    } else {
        let (heading, explanation) = if r.outcome == Outcome::Pending {
            crate::capture_context::pending(s).missing_answers()
        } else {
            (
                "No typed answers in this response",
                "Open Response to inspect the captured payload and any error details.",
            )
        };
        body = body
            .child(label(heading, 14.0, k.c.text_muted))
            .child(label(explanation, 12.0, k.c.text_faint).wrap());
    }
    let mut timing = col()
        .gap(12.0)
        .padding(16.0)
        .radius(8.0)
        .fill(k.c.bg_deep)
        .border(1.0, k.c.border_base)
        .child(
            row()
                .center_y()
                .child(label("Request timing", 13.0, k.c.text_base).grow(1.0))
                .child(ui::badge(
                    if r.outcome == Outcome::Pending {
                        pending_context(s).0.into()
                    } else {
                        format!("{:?}", r.outcome)
                    },
                    ui::status_color(r, k),
                    k,
                )),
        );
    if let (Some(headers), Some(total)) = (r.upstream_headers_ms, r.duration_ms) {
        let ratio = (headers / total.max(0.001)).clamp(0.0, 1.0) as f32;
        timing = timing
            .child(
                row()
                    .w_full()
                    .h(7.0)
                    .radius(3.0)
                    .clip_rounded(3.0)
                    .child(el().grow(ratio.max(0.001)).h_full().fill(k.c.text_accent))
                    .child(
                        el().grow((1.0 - ratio).max(0.001))
                            .h_full()
                            .fill(k.c.success.fg),
                    ),
            )
            .child(
                row()
                    .gap(12.0)
                    .child(
                        label(format!("Headers  {headers:.2} ms"), 11.0, k.c.text_accent).grow(1.0),
                    )
                    .child(label(
                        format!("Body  {:.2} ms", (total - headers).max(0.0)),
                        11.0,
                        k.c.success.fg,
                    )),
            );
    } else {
        if let Some(headers) = r.upstream_headers_ms {
            timing = timing.child(label(
                format!("Headers  {headers:.2} ms"),
                12.0,
                k.c.text_accent,
            ));
        }
        if let Some(total) = r.duration_ms {
            timing = timing.child(label(format!("Total  {total:.2} ms"), 12.0, k.c.text_base));
        }
        timing = timing.child(
            label(
                if r.outcome == Outcome::Pending {
                    pending_context(s).1
                } else {
                    "Complete timing was not recorded for this exchange."
                },
                12.0,
                k.c.text_muted,
            )
            .wrap(),
        );
    }
    timing = timing.child(
        label(
            "Relay-observed time includes forwarding and upstream work.",
            11.0,
            k.c.text_faint,
        )
        .wrap(),
    );
    body = body
        .child(timing)
        .child(
            row()
                .gap(16.0)
                .child(
                    fact("Model", r.model.as_deref().unwrap_or("Not captured"), k)
                        .grow(1.0)
                        .min_w(0.0),
                )
                .child(
                    fact("Started · UTC", &ui::timestamp(r.started_at_ms), k)
                        .grow(1.0)
                        .min_w(0.0),
                ),
        )
        .child(fact(
            "Request ID",
            r.request_id
                .as_deref()
                .unwrap_or("Not provided by upstream"),
            k,
        ));
    if let Some(note) = &r.note {
        body = body.child(label(note, 12.0, k.c.warning.fg).wrap());
    }
    if let Some(usage) = detail.response.json.as_ref().and_then(|j| j.get("usage")) {
        body = body.child(fact(
            "Token usage",
            &format!(
                "{} input · {} output",
                usage["input_tokens"], usage["output_tokens"]
            ),
            k,
        ));
    }
    body = body
        .child(ui::rule(k))
        .child(label("Capture details", 13.0, k.c.text_base));
    for (name, capture) in [("Request", &detail.request), ("Response", &detail.response)] {
        body = body.child(
            label(
                format!(
                    "{name} · {} captured of {} observed · {}",
                    ui::bytes(capture.captured_bytes as u64),
                    ui::bytes(capture.wire_bytes),
                    if capture.json.is_some() {
                        "JSON available"
                    } else {
                        "no JSON copy"
                    }
                ),
                11.0,
                k.c.text_muted,
            )
            .wrap(),
        );
        for note in &capture.notes {
            body = body.child(label(note, 11.0, k.c.warning.fg).wrap());
        }
    }
    body
}
pub fn summary(s: &State, r: &Summary, k: &Kit) -> El {
    col()
        .w_full()
        .gap(14.0)
        .child(
            row()
                .gap(10.0)
                .child(tile("Elapsed", elapsed(s, r), k))
                .child(tile("Sent", ui::bytes(r.request_bytes), k))
                .child(tile("Received", ui::bytes(r.response_bytes), k)),
        )
        .child(fact(
            "Model",
            r.model.as_deref().unwrap_or("Not captured"),
            k,
        ))
        .child(fact("Started · UTC", &ui::timestamp(r.started_at_ms), k))
        .child(fact(
            "Request ID",
            r.request_id
                .as_deref()
                .unwrap_or("Not provided by upstream"),
            k,
        ))
        .child(fact(
            "Upstream headers",
            &r.upstream_headers_ms
                .map(|v| format!("{v:.2} ms"))
                .unwrap_or_else(|| "Not recorded".into()),
            k,
        ))
        .child(fact(
            "Total duration",
            &r.duration_ms
                .map(|v| format!("{v:.2} ms"))
                .unwrap_or_else(|| "Not recorded".into()),
            k,
        ))
}
fn tile(name: &str, value: String, k: &Kit) -> El {
    let size = if value.chars().count() > 10 {
        14.0
    } else {
        20.0
    };
    col()
        .grow(1.0)
        .min_w(0.0)
        .gap(7.0)
        .padding(14.0)
        .radius(8.0)
        .fill(k.c.layer_02)
        .child(label(name.to_uppercase(), 10.0, k.c.text_muted))
        .child(ui::mono(value, size, k.c.text_base).wrap())
}
fn fact(name: &str, value: &str, k: &Kit) -> El {
    col()
        .gap(6.0)
        .child(label(name.to_uppercase(), 10.0, k.c.text_faint))
        .child(
            label(value, 12.0, k.c.text_base)
                .wrap_anywhere()
                .selectable(true),
        )
}
