use super::*;

pub fn view(detail: &Exchange, k: &Kit) -> El {
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
            .child(tile("Elapsed", ui::duration(r.duration_ms), k))
            .child(tile("Sent", ui::bytes(r.request_bytes), k))
            .child(tile("Received", ui::bytes(r.response_bytes), k)),
    );
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
                    format!("{:?}", r.outcome),
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
        timing = timing.child(
            label(
                "Timing becomes available as the exchange progresses.",
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
                    format!("{} questions", answers.len()),
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
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
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
                            label(clip(name, 100), 12.0, k.c.text_muted)
                                .wrap()
                                .grow(1.0),
                        )
                        .child(ui::badge(kind.to_uppercase(), k.c.text_accent, k)),
                )
                .child(
                    label(clip(&value, 500), 20.0, k.c.text_base)
                        .wrap()
                        .selectable(true),
                );
            if let Some(probabilities) = answer["probabilities"].as_object() {
                let mut values: Vec<_> = probabilities
                    .iter()
                    .filter_map(|(n, v)| v.as_f64().map(|v| (n, v)))
                    .collect();
                values.sort_by(|a, b| b.1.total_cmp(&a.1));
                for (name, probability) in values.into_iter().take(12) {
                    let p = probability.clamp(0.0, 1.0) as f32;
                    card = card.child(
                        row()
                            .gap(12.0)
                            .center_y()
                            .child(label(clip(name, 36), 11.0, k.c.text_muted).w(130.0))
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
                                ui::mono(
                                    format!("{:.1}%", probability * 100.0),
                                    11.0,
                                    k.c.text_base,
                                )
                                .w(55.0),
                            ),
                    );
                }
            }
            if let Some(confidence) = answer["confidence"].as_f64() {
                card = card.child(label(
                    format!("Confidence {:.1}%", confidence * 100.0),
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
        body = body
            .child(label(
                "No typed answers in this response",
                14.0,
                k.c.text_muted,
            ))
            .child(
                label(
                    "Open Response to inspect the captured payload and any error details.",
                    12.0,
                    k.c.text_faint,
                )
                .wrap(),
            );
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
fn tile(name: &str, value: String, k: &Kit) -> El {
    col()
        .grow(1.0)
        .min_w(0.0)
        .gap(7.0)
        .padding(14.0)
        .radius(8.0)
        .fill(k.c.layer_02)
        .child(label(name.to_uppercase(), 10.0, k.c.text_muted))
        .child(ui::mono(value, 20.0, k.c.text_base))
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
