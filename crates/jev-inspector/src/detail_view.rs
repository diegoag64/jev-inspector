use super::*;

pub fn detail_view(s: &State, k: &Kit, cx: &Cx<State>) -> El {
    let Some(id) = s.selected else {
        return col()
            .padding(28.0)
            .gap(14.0)
            .child(label("No request selected", 20.0, k.c.text_base).wrap())
            .child(label(
                "Select a request from the list to view its details.",
                14.0,
                k.c.text_muted,
            ));
    };
    let compact = s.workspace.window_height < 800.0;
    let mut pane = col()
        .min_h(0.0)
        .padding(if compact { 14.0 } else { 20.0 })
        .gap(if compact { 8.0 } else { 14.0 });
    let summary = s.snapshot.requests.iter().find(|r| r.id == id);
    pane = pane.child(
        row()
            .center_y()
            .gap(12.0)
            .child(label(format!("Exchange #{id:03}"), 20.0, k.c.text_base).grow(1.0))
            .child(ui::badge(
                summary.map(status).unwrap_or_else(|| "Evicted".into()),
                summary
                    .map(|r| ui::status_color(r, k))
                    .unwrap_or(k.c.text_muted),
                k,
            ))
            .child(
                ui::action(
                    if s.workspace.starred.contains(&id) {
                        &rune_kit::glyphs::STAR_FILLED
                    } else {
                        &rune_kit::glyphs::STAR
                    },
                    if s.workspace.starred.contains(&id) {
                        "Unstar exchange"
                    } else {
                        "Star exchange"
                    },
                    s.workspace.starred.contains(&id),
                )
                .shortcut("cmd+b")
                .on_press(State::star)
                .build(k)
                .key("star-exchange")
                .checked(s.workspace.starred.contains(&id)),
            )
            .child(
                ui::action(
                    if s.workspace.detail_expanded {
                        &icons::RESTORE
                    } else {
                        &icons::EXPAND
                    },
                    if s.workspace.detail_expanded {
                        "Restore split view"
                    } else {
                        "Expand exchange"
                    },
                    false,
                )
                .on_press(State::toggle_expanded)
                .build(k)
                .key("expand-detail"),
            ),
    );
    if let Some(r) = summary {
        pane = pane.child(
            row()
                .center_y()
                .gap(8.0)
                .child(
                    ui::mono(format!("{} {}", r.method, r.path), 12.0, k.c.text_muted)
                        .truncate_middle()
                        .help(format!("{} {}", r.method, r.path))
                        .grow(1.0)
                        .min_w(0.0)
                        .selectable(true),
                )
                .child(
                    ui::action(&icons::UP, "Previous request", false)
                        .shortcut("Alt+Up")
                        .disabled(s.visible.first().is_none_or(|r| r.id == id))
                        .on_press_tx(|s: &mut State, tx| s.navigate(-1, tx))
                        .build(k)
                        .key("previous-request"),
                )
                .child(
                    ui::action(&icons::DOWN, "Next request", false)
                        .shortcut("Alt+Down")
                        .disabled(s.visible.last().is_none_or(|r| r.id == id))
                        .on_press_tx(|s: &mut State, tx| s.navigate(1, tx))
                        .build(k)
                        .key("next-request"),
                )
                .child(
                    ui::action(&rune_kit::glyphs::COPY, "Copy request ID", false)
                        .disabled(r.request_id.is_none())
                        .on_press_tx(|s: &mut State, tx| {
                            if let Some(id) = s
                                .snapshot
                                .requests
                                .iter()
                                .find(|r| Some(r.id) == s.selected)
                                .and_then(|r| r.request_id.clone())
                            {
                                tx.copy(id);
                                s.workspace.notice = Some(("Request ID copied".into(), false));
                            }
                        })
                        .build(k)
                        .key("copy-id"),
                ),
        );
    }
    pane = pane.child(
        row()
            .key("detail-tabs")
            .role("tablist")
            .label("Exchange details")
            .focus_group()
            .gap(6.0)
            .child(tab("Overview", 0, s.tab, k))
            .child(tab("Request", 1, s.tab, k))
            .child(tab("Response", 2, s.tab, k))
            .child(tab("Diagram", 3, s.tab, k))
            .child(tab("Compare", 4, s.tab, k))
            .child(el().grow(1.0)),
    );
    pane = pane.child(ui::rule(k));
    if s.detail.is_some()
        && let Some(error) = &s.detail_error
    {
        pane = pane.child(
            label(
                format!("Showing the last received copy. {error}"),
                11.0,
                k.c.warning.fg,
            )
            .wrap(),
        );
    }
    pane.child(content(s, k, cx))
}

fn content(s: &State, k: &Kit, cx: &Cx<State>) -> El {
    let Some(detail) = &s.detail else {
        return motion::detail(s, detail_states::missing(s, k));
    };
    if s.tab == 2 && detail.response.json.is_none() {
        if detail.summary.outcome == Outcome::Pending && !detail.response.complete {
            return motion::detail(s, detail_states::waiting_response(s, k));
        }
        if detail.response.complete
            && detail.response.wire_bytes == 0
            && detail.response.notes.is_empty()
        {
            return motion::detail(s, detail_states::empty_body(s, k));
        }
    }
    if s.tab == 3 {
        return motion::detail(
            s,
            diagram::view(s, detail, k)
                .role("tabpanel")
                .label("Diagram"),
        );
    }
    if s.tab == 4 {
        return motion::detail(
            s,
            compare::view(s, detail, k, cx)
                .role("tabpanel")
                .label("Compare"),
        );
    }
    let mut body = col().gap(14.0).w_full();
    if s.tab == 0 {
        body = crate::overview::view(detail, k);
    } else {
        let (capture, preview) = if s.tab == 1 {
            (&detail.request, &s.request_text)
        } else {
            (&detail.response, &s.response_text)
        };
        body = body.child(label(
            format!(
                "{} · {} bytes · {}",
                capture.content_type,
                capture.wire_bytes,
                if capture.encoding.is_empty() {
                    "identity"
                } else {
                    &capture.encoding
                }
            ),
            11.0,
            k.c.text_muted,
        ));
        for note in &capture.notes {
            body = body.child(label(note, 12.0, k.c.warning.fg).wrap());
        }
        let (key, get): (&str, fn(&mut State) -> &mut json_view::JsonDocument) = if s.tab == 1 {
            ("request-json", |s| &mut s.request_text)
        } else {
            ("response-json", |s| &mut s.response_text)
        };
        return motion::detail(
            s,
            body.role("tabpanel")
                .label(if s.tab == 1 { "Request" } else { "Response" })
                .grow(1.0)
                .min_h(0.0)
                .child(preview.view(key, get, k, s, cx)),
        );
    }
    motion::detail(
        s,
        request_scroll::area(
            cx,
            k,
            "detail-scroll",
            "overview-content",
            col()
                .role("tabpanel")
                .label("Overview")
                .key("detail-scroll")
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .focusable(true)
                .child(body.key("overview-content")),
            false,
        ),
    )
}

fn tab(name: &str, value: u8, current: u8, k: &Kit) -> El {
    Button::new(name)
        .variant(if value == current {
            ButtonVariant::Contrast
        } else {
            ButtonVariant::Ghost
        })
        .on_press(move |s: &mut State| {
            showcase::stop(s);
            s.tab = value;
        })
        .build(k)
        .role("tab")
        .focus_selected(value == current)
        .key(format!("tab-{value}"))
}
