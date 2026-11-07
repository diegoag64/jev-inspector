use super::*;

fn with_partial_response_note(s: &State, detail: &Exchange, k: &Kit, content: El) -> El {
    if detail.summary.outcome != Outcome::Pending
        || detail.response.complete
        || detail.response.json.is_none()
    {
        return content;
    }
    if s.tab == 0
        && detail
            .response
            .json
            .as_ref()
            .and_then(|v| v.get("answers"))
            .and_then(serde_json::Value::as_object)
            .is_none()
    {
        return content;
    }
    col()
        .w_full()
        .gap(6.0)
        .child(
            label(
                crate::capture_context::pending(s).partial_notice(),
                11.0,
                k.c.warning.fg,
            )
            .wrap()
            .role("status")
            .key("partial-response-note"),
        )
        .child(content)
}

fn compact_header(s: &State, summary: Option<&Summary>, id: u64, k: &Kit) -> El {
    let mut title = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(4.0)
        .child(label(format!("Exchange #{id:03}"), 18.0, k.c.text_base).truncate());
    if let Some(r) = summary {
        let endpoint = format!("{} {}", r.method, r.path);
        title = title.child(
            ui::mono(&endpoint, 11.0, k.c.text_muted)
                .truncate_middle()
                .help(endpoint)
                .selectable(true),
        );
    }
    let mut header = row().center_y().gap(4.0).child(title);
    if let Some(r) = summary {
        header = header.child(ui::badge(
            crate::capture_context::status(s, r),
            ui::status_color(r, k),
            k,
        ));
    }
    header = header.child(
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
        .control_size(rune_kit::ControlSize::Large)
        .shortcut("cmd+b")
        .on_press(State::star)
        .build(k)
        .key("star-exchange")
        .checked(s.workspace.starred.contains(&id)),
    );
    for (glyph, name, key, direction, disabled) in [
        (
            &icons::UP,
            "Previous request",
            "previous-request",
            -1,
            s.visible.first().is_none_or(|r| r.id == id),
        ),
        (
            &icons::DOWN,
            "Next request",
            "next-request",
            1,
            s.visible.last().is_none_or(|r| r.id == id),
        ),
    ] {
        header = header.child(
            ui::action(glyph, name, false)
                .control_size(rune_kit::ControlSize::Large)
                .shortcut(if direction < 0 { "Alt+Up" } else { "Alt+Down" })
                .disabled(disabled)
                .on_press_tx(move |s: &mut State, tx| s.navigate(direction, tx))
                .build(k)
                .key(key),
        );
    }
    header = header.child(
        ui::action(&rune_kit::glyphs::COPY, "Copy request ID", false)
            .control_size(rune_kit::ControlSize::Large)
            .disabled(summary.and_then(|r| r.request_id.as_ref()).is_none())
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
    );
    header.child(
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
        .control_size(rune_kit::ControlSize::Large)
        .on_press(State::toggle_expanded)
        .build(k)
        .key("expand-detail"),
    )
}

pub fn detail_view(s: &State, k: &Kit, cx: &Cx<State>, layers: &mut Vec<El>) -> El {
    let Some(id) = s.selected else {
        if s.snapshot.requests.is_empty() && s.workspace.session_name.is_none() {
            return setup_view::panel(s, k, cx);
        }
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
        .gap(if compact { 6.0 } else { 14.0 });
    let summary = s.snapshot.requests.iter().find(|r| r.id == id);
    if compact {
        pane = pane.child(compact_header(s, summary, id, k));
    } else {
        pane = pane.child(
            row()
                .center_y()
                .gap(12.0)
                .child(label(format!("Exchange #{id:03}"), 20.0, k.c.text_base).grow(1.0))
                .child(ui::badge(
                    summary
                        .map(|r| crate::capture_context::status(s, r))
                        .unwrap_or_else(|| "Evicted".into()),
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
    }
    if !s.visible.iter().any(|r| r.id == id) {
        pane = pane.child(
            row()
                .center_y()
                .gap(8.0)
                .child(
                    label(
                        "Selected exchange is outside the visible results",
                        12.0,
                        k.c.warning.fg,
                    )
                    .wrap()
                    .grow(1.0),
                )
                .child(ui::button(
                    "Show in list",
                    "show-selected",
                    false,
                    k,
                    State::reset_filters,
                )),
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
    pane.child(content(s, k, cx, layers))
}

fn content(s: &State, k: &Kit, cx: &Cx<State>, layers: &mut Vec<El>) -> El {
    let Some(detail) = &s.detail else {
        if s.tab == 0
            && (s.workspace.session_name.is_some()
                || s.detail_error.is_some()
                || s.connection_error.is_some())
            && let Some(summary) = s
                .snapshot
                .requests
                .iter()
                .find(|r| Some(r.id) == s.selected)
        {
            let title = if s.workspace.session_name.is_some() {
                "Summary only"
            } else if s.connection_error.is_some() {
                "Relay unavailable"
            } else {
                "Couldn't load request details"
            };
            let mut body = col().gap(14.0).child(label(title, 15.0, k.c.warning.fg).key("detail-unavailable"))
                .child(label("Captured bodies are unavailable. The recorded request metadata is still available below.", 12.0, k.c.text_muted).wrap());
            if let Some(error) = &s.connection_error {
                body = body.child(
                    label(
                        format!("{error} Reconnecting automatically."),
                        12.0,
                        k.c.text_muted,
                    )
                    .wrap(),
                );
            }
            if s.workspace.session_name.is_none() && s.connected {
                body = body.child(ui::button("Retry loading", "retry-detail", true, k, |s| {
                    s.detail_error = None;
                    s.mailbox.lock().unwrap().refresh = true;
                }));
            }
            return scroll_overview(s, body.child(overview::summary(s, summary, k)), k, cx);
        }
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
            with_partial_response_note(s, detail, k, diagram::view(s, detail, k, layers))
                .grow(1.0)
                .min_h(0.0)
                .role("tabpanel")
                .label("Diagram"),
        );
    }
    if s.tab == 4 {
        return motion::detail(
            s,
            with_partial_response_note(s, detail, k, compare::view(s, detail, k, cx))
                .grow(1.0)
                .min_h(0.0)
                .role("tabpanel")
                .label("Compare"),
        );
    }
    let mut body = col()
        .gap(if s.workspace.window_height < 800.0 {
            8.0
        } else {
            14.0
        })
        .w_full();
    if s.tab == 0 {
        body = with_partial_response_note(s, detail, k, crate::overview::view(s, detail, k));
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
        if !capture.complete && s.workspace.session_name.is_some() {
            body = body.child(
                label(
                    "Incomplete when saved · This snapshot will not update.",
                    12.0,
                    k.c.warning.fg,
                )
                .wrap()
                .role("status"),
            );
        }
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
    scroll_overview(s, body, k, cx)
}

fn scroll_overview(s: &State, body: El, k: &Kit, cx: &Cx<State>) -> El {
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
            s.select_tab(value);
        })
        .build(k)
        .role("tab")
        .focus_selected(value == current)
        .key(format!("tab-{value}"))
}
