use super::*;
use rune_kit::{Cell, ControlSize, Dialog, Table, TableColumn, TableStyle, TextInput};
use workspace::Filter;

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let k = ui::kit(s.dark);
    let header = row()
        .h(44.0)
        .px(14.0)
        .gap(9.0)
        .center_y()
        .child(
            el().size(24.0, 24.0)
                .center()
                .radius(5.0)
                .fill(k.alpha(k.c.text_base, 0.08))
                .child(label("j", 19.0, k.c.text_base)),
        )
        .child(label("jev", 16.0, k.c.text_base))
        .child(label("inspector", 14.0, k.c.text_muted))
        .child(el().grow(1.0))
        .child(ui::badge(
            if s.workspace.session_name.is_some() {
                "SAVED SESSION"
            } else if s.showcase.is_some() {
                "EXAMPLES"
            } else if s.demo {
                "SYNTHETIC FIXTURE"
            } else if s.connected {
                "LOCAL CONNECTED"
            } else {
                "RECONNECTING"
            },
            if s.connected || s.demo {
                k.c.success.fg
            } else {
                k.c.warning.fg
            },
            &k,
        ))
        .child(
            if s.workspace.window_width >= 1300.0 || s.showcase.is_none() {
                Button::new("Open")
                    .icon(&icons::OPEN)
                    .variant(ButtonVariant::Ghost)
                    .control_size(ControlSize::Large)
                    .help("Open a saved session (Ctrl/Cmd+O)")
                    .on_press_tx(|s: &mut State, tx| session::open_dialog(s, tx))
                    .build(&k)
                    .label("Open session")
                    .key("open-session")
            } else {
                ui::action(&icons::OPEN, "Open session", false)
                    .help("Open a saved session")
                    .shortcut("cmd+o")
                    .on_press_tx(|s: &mut State, tx| session::open_dialog(s, tx))
                    .build(&k)
                    .key("open-session")
            },
        )
        .child(
            if s.workspace.window_width >= 1300.0 || s.showcase.is_none() {
                Button::new("Export")
                    .icon(&icons::EXPORT)
                    .variant(if s.workspace.export {
                        ButtonVariant::Contrast
                    } else {
                        ButtonVariant::Ghost
                    })
                    .control_size(ControlSize::Large)
                    .help("Save session, exchange JSON, or CSV (Ctrl/Cmd+E)")
                    .on_press(|s: &mut State| s.workspace.export = true)
                    .build(&k)
                    .key("export-menu")
            } else {
                ui::action(&icons::EXPORT, "Export", s.workspace.export)
                    .help("Save session, exchange JSON, or CSV")
                    .shortcut("cmd+e")
                    .on_press(|s: &mut State| s.workspace.export = true)
                    .build(&k)
                    .key("export-menu")
            },
        )
        .child(
            ui::action(&icons::HELP, "Help", s.workspace.help)
                .help("Search syntax and keyboard shortcuts")
                .shortcut("F1")
                .on_press(|s: &mut State| s.workspace.help = true)
                .build(&k)
                .key("help"),
        )
        .child(
            ui::action(
                if s.dark { &icons::SUN } else { &icons::MOON },
                if s.dark {
                    "Switch to light theme"
                } else {
                    "Switch to dark theme"
                },
                false,
            )
            .on_press(State::toggle_theme)
            .build(&k)
            .key("theme-toggle"),
        );
    let mut root = col()
        .key("workspace")
        .on_key(|s: &mut State, event, _| {
            if event.key == rune::Key::Tab
                && s.tab == 3
                && !s.workspace.help
                && !s.workspace.export
                && !s.cost.settings
            {
                diagram::reveal_keyboard_card(s);
            }
            false
        })
        .on_layout_tx(|s: &mut State, r, tx| {
            s.workspace.window_width = r[2];
            s.workspace.window_height = r[3];
            tx.instant();
        })
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .child(header)
        .child(ui::rule(&k));
    if let Some(name) = &s.workspace.session_name {
        root = root.child(
            row()
                .px(22.0)
                .py(8.0)
                .center_y()
                .gap(12.0)
                .fill(k.c.layer_02)
                .child(
                    label(
                        format!(
                            "{} · {name}",
                            if s.workspace.last_received {
                                "Last-received snapshot"
                            } else {
                                "Offline session"
                            }
                        ),
                        12.0,
                        k.c.text_accent,
                    )
                    .grow(1.0)
                    .min_w(0.0)
                    .truncate_middle()
                    .help(name),
                )
                .child(ui::button(
                    if s.demo {
                        "Return to demo"
                    } else {
                        "Return to live"
                    },
                    "return-live",
                    false,
                    &k,
                    State::return_live,
                )),
        );
    } else if let Some(error) = &s.connection_error {
        root = root.child(
            label(error, 12.0, k.c.warning.fg)
                .wrap()
                .px(22.0)
                .py(10.0)
                .fill(k.c.layer_02),
        );
    }
    if let Some((message, error)) = &s.workspace.notice {
        root = root.child(
            row()
                .px(22.0)
                .py(6.0)
                .gap(12.0)
                .center_y()
                .fill(k.c.layer_02)
                .child(
                    label(
                        message,
                        12.0,
                        if *error {
                            k.c.danger.fg
                        } else {
                            k.c.success.fg
                        },
                    )
                    .role(if *error { "alert" } else { "status" })
                    .wrap()
                    .grow(1.0)
                    .min_w(0.0)
                    .selectable(true),
                )
                .child(
                    ui::action(&rune_kit::glyphs::CLOSE, "Dismiss notification", false)
                        .on_press(|s: &mut State| s.workspace.notice = None)
                        .build(&k)
                        .key("dismiss-notice"),
                ),
        );
    }
    if let Some(job) = &s.workspace.job {
        use std::sync::atomic::Ordering;
        root = root.child(
            row()
                .px(22.0)
                .py(6.0)
                .gap(12.0)
                .center_y()
                .fill(k.c.layer_02)
                .child(
                    label(
                        format!(
                            "Working… {} / {} exchanges",
                            job.done.load(Ordering::Relaxed),
                            job.total.load(Ordering::Relaxed)
                        ),
                        12.0,
                        k.c.text_accent,
                    )
                    .grow(1.0),
                )
                .child(ui::button("Cancel", "cancel-job", false, &k, |s| {
                    if let Some(job) = &s.workspace.job {
                        job.cancel.store(true, Ordering::Relaxed);
                    }
                })),
        );
    }
    let expanded = s.workspace.detail_expanded;
    if !expanded {
        root = root.child(metrics(s, &k));
    }
    let mut body = row().grow(1.0).min_h(0.0).px(8.0).pb(6.0).gap(3.0);
    if !expanded {
        body = body.child(traffic(s, &k, cx)).child(splitter(&k));
    }
    let mut layers = vec![];
    body = body.child(
        detail_view::detail_view(s, &k, cx, &mut layers)
            .grow(1.0)
            .min_w(0.0)
            .h_full()
            .radius(7.0)
            .border(1.0, k.c.border_base)
            .fill(k.c.layer_01),
    );
    root = root.child(body).child(footer(s, &k));
    dialogs(s, &k, &mut layers);
    cost::dialog(s, &k, &mut layers);
    root.children(layers)
}

fn splitter(k: &Kit) -> El {
    el().key("splitter")
        .w(6.0)
        .h_full()
        .radius(3.0)
        .hover(k.alpha(k.c.text_accent, 0.3))
        .cursor(rune::Cursor::ResizeColumn)
        .focusable(true)
        .focus_ring(1.0, -1.0, k.c.text_faint)
        .role("separator")
        .label("Resize request list")
        .help("Drag to resize · double-click to reset · Left/Right when focused")
        .on_double_press(|s: &mut State| s.workspace.list_width = 550.0)
        .on_drag(|s: &mut State, d, _| match d.phase {
            rune::Phase::Down => {
                d.data = s
                    .workspace
                    .list_width
                    .min((s.workspace.window_width - 530.0).max(440.0))
            }
            rune::Phase::Move => {
                s.workspace.list_width = (d.data + d.dx).clamp(
                    440.0,
                    (s.workspace.window_width - 530.0).clamp(440.0, 800.0),
                )
            }
            rune::Phase::Up => {
                if d.cancelled() {
                    s.workspace.list_width = d.data;
                } else if let Err(e) = s.workspace.prefs.set("list_width", &s.workspace.list_width)
                {
                    s.workspace.notice = Some((e.to_string(), true));
                }
            }
        })
        .on_key(|s: &mut State, ev, _| {
            let dx = match ev.key {
                rune::Key::Left => -20.0,
                rune::Key::Right => 20.0,
                _ => return false,
            };
            s.workspace.list_width = (s.workspace.list_width + dx).clamp(
                440.0,
                (s.workspace.window_width - 530.0).clamp(440.0, 800.0),
            );
            true
        })
}

fn metrics(s: &State, k: &Kit) -> El {
    let mut times: Vec<_> = s.visible.iter().filter_map(|r| r.duration_ms).collect();
    times.sort_by(f64::total_cmp);
    let quantile = |p: f64| {
        times
            .get(((times.len() as f64 * p).ceil() as usize).saturating_sub(1))
            .copied()
    };
    let failures = s.visible.iter().filter(|r| failed(r)).count();
    let pending = s
        .visible
        .iter()
        .filter(|r| r.outcome == Outcome::Pending)
        .count();
    let mut bar = row().px(16.0).h(34.0).center_y().gap(18.0);
    for (title, value, sub, color) in [
        (
            "OBSERVED",
            s.snapshot.total_requests.to_string(),
            "relay lifetime",
            k.c.text_base,
        ),
        (
            "VISIBLE",
            s.visible.len().to_string(),
            "current filters",
            k.c.text_base,
        ),
        (
            "ERRORS",
            failures.to_string(),
            "visible requests",
            if failures > 0 {
                k.c.danger.fg
            } else {
                k.c.text_base
            },
        ),
        (
            crate::capture_context::pending(s).count_label(),
            pending.to_string(),
            "visible requests",
            k.c.warning.fg,
        ),
        (
            "MEDIAN",
            if times.is_empty() {
                "—".into()
            } else {
                ui::duration(quantile(0.5))
            },
            "visible elapsed",
            k.c.text_base,
        ),
        (
            "P95",
            if times.is_empty() {
                "—".into()
            } else {
                ui::duration(quantile(0.95))
            },
            "visible elapsed",
            k.c.text_base,
        ),
    ] {
        bar = bar.child(
            row()
                .grow(1.0)
                .gap(7.0)
                .center_y()
                .child(label(title, 10.0, k.c.text_faint))
                .child(motion::metric(title, value, 13.0, color, k))
                .help(format!("{title}: {sub}")),
        );
    }
    bar
}
fn traffic(s: &State, k: &Kit, cx: &Cx<State>) -> El {
    let mut panel = col()
        .w(s.workspace
            .list_width
            .min((s.workspace.window_width - 530.0).max(440.0)))
        .h_full()
        .min_h(0.0)
        .radius(7.0)
        .clip_rounded(7.0)
        .border(1.0, k.c.border_base)
        .fill(k.c.layer_01)
        .child(
            row()
                .px(10.0)
                .py(7.0)
                .center_y()
                .gap(6.0)
                .child(label("Traffic", 15.0, k.c.text_base).grow(1.0))
                .child(
                    ui::action(
                        if s.workspace.paused {
                            &icons::PLAY
                        } else {
                            &icons::PAUSE
                        },
                        if s.workspace.paused {
                            "Resume live updates"
                        } else {
                            "Pause live updates"
                        },
                        s.workspace.paused,
                    )
                    .help(if s.demo || s.workspace.session_name.is_some() {
                        "Live updates are unavailable in a saved session or static fixture"
                    } else if s.workspace.paused {
                        "Resume the list with the latest captured requests"
                    } else {
                        "Freeze the list; the relay keeps recording"
                    })
                    .shortcut("cmd+p")
                    .disabled(s.demo || s.workspace.session_name.is_some())
                    .on_press(State::toggle_pause)
                    .build(k)
                    .key("pause")
                    .checked(s.workspace.paused),
                )
                .child(
                    Button::new(if s.workspace.follow {
                        "Following"
                    } else {
                        "Follow"
                    })
                    .icon(&icons::FOLLOW)
                    .control_size(ControlSize::Large)
                    .variant(if s.workspace.follow {
                        ButtonVariant::Contrast
                    } else {
                        ButtonVariant::Ghost
                    })
                    .help(if s.workspace.follow {
                        "Stop following the newest matching request"
                    } else {
                        "Select the newest matching request as it arrives"
                    })
                    .on_press(|s: &mut State| {
                        s.workspace.follow = !s.workspace.follow;
                        s.table.clear_sort();
                        s.filter_rows();
                    })
                    .build(k)
                    .key("follow")
                    .checked(s.workspace.follow),
                )
                .child(
                    Button::new("Clear view")
                        .icon(&icons::CLEAR)
                        .control_size(ControlSize::Large)
                        .variant(ButtonVariant::Ghost)
                        .help("Hide current rows in this window; restore them at any time")
                        .shortcut("cmd+l")
                        .on_press(State::clear_view)
                        .build(k)
                        .key("clear-view"),
                ),
        )
        .child(
            col().w_full().px(10.0).child(
                TextInput::new(&s.filter)
                    .width(None)
                    .placeholder("Search requests…  status:4xx  duration:>200")
                    .leading_icon(k.glyph.search)
                    .label("Search request metadata")
                    .clear_button()
                    .height(30.0)
                    .invalid(s.workspace.query_error.is_some())
                    .on_edit(|s: &mut State| &mut s.filter)
                    .on_change(|s: &mut State, _| s.filter_rows())
                    .on_key(workspace::on_key)
                    .build(k)
                    .key("request-filter"),
            ),
        );
    if let Some(error) = &s.workspace.query_error {
        panel = panel.child(
            label(
                format!("{error} Showing previous results until the query is valid."),
                11.0,
                k.c.danger.fg,
            )
            .wrap()
            .px(10.0)
            .py(5.0),
        );
    }
    panel = panel.child(
        row()
            .px(10.0)
            .py(6.0)
            .gap(3.0)
            .child(ui::button(
                "All",
                "all-filter",
                !s.errors_only && s.workspace.filter == Filter::All,
                k,
                |s| {
                    s.errors_only = false;
                    s.workspace.filter = Filter::All;
                    s.filter_rows();
                },
            ))
            .child(ui::button(
                "Errors",
                "errors-filter",
                s.errors_only,
                k,
                |s| {
                    s.errors_only = !s.errors_only;
                    s.workspace.filter = Filter::All;
                    s.filter_rows();
                },
            ))
            .child(ui::button(
                "Success",
                "success-filter",
                s.workspace.filter == Filter::Success,
                k,
                |s| {
                    s.errors_only = false;
                    s.workspace.filter = Filter::Success;
                    s.filter_rows();
                },
            ))
            .child(ui::button(
                crate::capture_context::pending(s).filter_label(),
                "pending-filter",
                s.workspace.filter == Filter::Pending,
                k,
                |s| {
                    s.errors_only = false;
                    s.workspace.filter = Filter::Pending;
                    s.filter_rows();
                },
            ))
            .child(ui::button(
                "Starred",
                "starred-filter",
                s.workspace.filter == Filter::Starred,
                k,
                |s| {
                    s.errors_only = false;
                    s.workspace.filter = Filter::Starred;
                    s.filter_rows();
                },
            )),
    );
    if s.workspace.paused {
        let newer = s.workspace.pending.as_ref().map_or(0, |p| {
            p.total_requests.saturating_sub(s.snapshot.total_requests)
        });
        panel = panel.child(
            label(
                format!("List paused · {newer} new requests · relay still recording"),
                11.0,
                k.c.warning.fg,
            )
            .px(10.0)
            .py(5.0)
            .fill(k.c.layer_02),
        );
    }
    if s.workspace.clear_before > 0 && !s.visible.is_empty() {
        panel = panel.child(
            row()
                .px(10.0)
                .py(5.0)
                .center_y()
                .child(label("Earlier requests hidden", 11.0, k.c.text_muted).grow(1.0))
                .child(ui::button("Restore", "restore-view", false, k, |s| {
                    s.workspace.clear_before = 0;
                    s.filter_rows();
                })),
        );
    }
    if s.cost.enabled {
        panel = panel.child(cost::summary(s, k));
    }
    let list = if s.visible.is_empty() {
        empty(s, k)
    } else {
        table(s, k, cx)
    };
    panel
        .child(ui::rule(k))
        .child(list.grow(1.0).min_h(0.0))
        .child(ui::rule(k))
        .child(
            row()
                .px(12.0)
                .py(5.0)
                .center_y()
                .gap(8.0)
                .child(
                    label(
                        format!(
                            "{} of {} retained",
                            s.visible.len(),
                            s.snapshot.requests.len()
                        ),
                        11.0,
                        k.c.text_muted,
                    )
                    .grow(1.0),
                )
                .child(
                    ui::button("Costs", "cost-mode", s.cost.enabled, k, cost::toggle)
                        .checked(s.cost.enabled)
                        .help("Show estimated cost and token usage for filtered Traffic"),
                )
                .child(
                    ui::button(
                        if s.workspace.compact {
                            "Comfortable"
                        } else {
                            "Compact"
                        },
                        "density",
                        false,
                        k,
                        State::toggle_density,
                    )
                    .help("Change row density"),
                ),
        )
}
fn table(s: &State, k: &Kit, cx: &Cx<State>) -> El {
    let mut style = TableStyle::from_kit(k);
    style.row_height = if s.workspace.compact { 30.0 } else { 48.0 };
    style.header_height = 29.0;
    style.cell_pad = 9.0;
    style.rule = k.c.border_base;
    // Opaque selection surfaces keep supporting text and semantic status readable.
    style.selected = rune::color::hex(if k.dark { "#303030" } else { "#ededed" }).unwrap();
    style.muted = rune::color::hex(if k.dark { "#b8b8b8" } else { "#5c5c5c" }).unwrap();
    let supporting = style.muted;
    let table_kit = k.clone();
    Table::new(
        |s: &State| &s.table,
        |s| &mut s.table,
        |s| s.visible.len(),
        |s, i| s.visible[i].id.to_string().into(),
    )
    .column(
        TableColumn::new("star", "")
            .width(33.0)
            .min(33.0)
            .resizable(false),
    )
    .column(
        TableColumn::new("request", "Request")
            .flex(1.0)
            .min(135.0)
            .sortable(),
    )
    .column(
        TableColumn::new("status", "Status")
            .width(74.0)
            .min(65.0)
            .sortable(),
    )
    .column(
        TableColumn::new(
            if s.cost.enabled { "tokens" } else { "time" },
            if s.cost.enabled {
                "Input tokens"
            } else {
                "Elapsed"
            },
        )
        .width(if s.cost.enabled { 108.0 } else { 82.0 })
        .min(if s.cost.enabled { 100.0 } else { 70.0 })
        .sortable(),
    )
    .column(
        TableColumn::new(
            if s.cost.enabled { "cost" } else { "started" },
            if s.cost.enabled {
                "Est. USD"
            } else {
                "Time · UTC"
            },
        )
        .width(105.0)
        .min(85.0)
        .sortable(),
    )
    .cell(move |s, i, column| {
        let r = &s.visible[i];
        let k = &table_kit;
        match column {
            "star" => {
                let starred = s.workspace.starred.contains(&r.id);
                let mut cell = row().h(24.0).center_y().child(label(
                    if starred { "\u{2605}" } else { "\u{00b7}" },
                    12.0,
                    if starred {
                        k.c.text_accent
                    } else {
                        k.c.text_faint
                    },
                ));
                if s.selected == Some(r.id) {
                    cell = cell.child(
                        el().key("request-selection-marker")
                            .matched("request-selection-marker")
                            .absolute(-7.0, 1.0)
                            .size(3.0, 22.0)
                            .radius(1.5)
                            .fill(k.c.text_base)
                            .motion(rune::Spring::new(0.24, 0.0))
                            .a11y_hidden(),
                    );
                }
                Cell::el(cell)
            }
            "request" if !s.workspace.compact => Cell::el(
                col()
                    .gap(4.0)
                    .child(label(
                        format!("{}  {}", r.method, r.path),
                        12.0,
                        k.c.text_base,
                    ))
                    .child(label(
                        format!(
                            "#{:03} · {}",
                            r.id,
                            r.model.as_deref().unwrap_or("Model unavailable")
                        ),
                        10.0,
                        supporting,
                    )),
            ),
            "request" => Cell::el(
                row()
                    .center_y()
                    .gap(8.0)
                    .child(ui::mono(format!("#{:03}", r.id), 11.0, supporting).w(34.0))
                    .child(
                        label(
                            r.model
                                .clone()
                                .unwrap_or_else(|| format!("{} {}", r.method, r.path)),
                            12.0,
                            k.c.text_base,
                        )
                        .grow(1.0)
                        .min_w(0.0)
                        .truncate(),
                    )
                    .help(format!(
                        "{} {} · {}",
                        r.method,
                        r.path,
                        r.model.as_deref().unwrap_or("Model unavailable")
                    )),
            ),
            "status" => Cell::text(if r.outcome == Outcome::Pending {
                "Pending".into()
            } else {
                status(r)
            })
            .color(ui::status_color(r, k))
            .mono(),
            "time" => Cell::text(
                r.duration_ms
                    .map(|ms| ui::duration(Some(ms)))
                    .unwrap_or_else(|| "—".into()),
            )
            .mono()
            .muted(),
            "started" => Cell::text(ui::time(r.started_at_ms)).mono().muted(),
            "tokens" => Cell::text(
                cost::usage(s, r)
                    .and_then(|u| u.input_tokens)
                    .map_or("—".into(), |v| v.to_string()),
            )
            .mono()
            .muted(),
            "cost" => Cell::el(
                ui::mono(cost::estimate(s, r).display(), 12.0, k.c.text_base)
                    .help(cost::row_help(s, r)),
            ),
            _ => Cell::text(""),
        }
    })
    .on_sort(|s, _, _, _| {
        s.workspace.follow = false;
        s.filter_rows();
    })
    .on_select(|s, id, _| {
        if let Ok(id) = id.parse() {
            s.workspace.follow = false;
            s.select(id);
        }
    })
    .zebra(false)
    .style(style)
    .name("Captured requests")
    .build(s, k)
    .focus_ring(1.0, -1.0, k.c.text_faint)
    .pr(16.0)
    .child(request_scroll::view(cx, k, style.header_height))
}
fn empty(s: &State, k: &Kit) -> El {
    let waiting = s.snapshot.requests.is_empty() && s.workspace.session_name.is_none();
    let empty_session = s.snapshot.requests.is_empty() && s.workspace.session_name.is_some();
    let cleared = s.workspace.clear_before > 0;
    let mut body = col()
        .padding(24.0)
        .gap(10.0)
        .child(
            label(
                if cleared {
                    "View cleared"
                } else if waiting {
                    "No requests yet"
                } else if empty_session {
                    "No requests in this session"
                } else {
                    "No matching requests"
                },
                18.0,
                k.c.text_base,
            )
            .wrap(),
        )
        .child(
            label(
                if cleared {
                    "Earlier requests are hidden. Restore them to show them again."
                } else if waiting {
                    "Send a request through the local relay to see it here."
                } else if empty_session {
                    "Open another session or return to live traffic."
                } else {
                    "Change your search or reset the filters."
                },
                13.0,
                k.c.text_muted,
            )
            .wrap(),
        );
    if waiting {
        body = body.child(ui::button("Connection setup", "open-setup", true, k, |s| {
            s.workspace.setup_help = true;
            s.workspace.help = true;
        }));
        body = body.child(
            ui::mono(
                if s.snapshot.relay_url.starts_with("http") {
                    &s.snapshot.relay_url
                } else {
                    "http://127.0.0.1:8787"
                },
                13.0,
                k.c.text_accent,
            )
            .wrap()
            .selectable(true),
        );
    } else if cleared {
        body = body.child(row().child(ui::button(
            "Restore requests",
            "empty-restore",
            true,
            k,
            |s| {
                s.workspace.clear_before = 0;
                s.filter_rows();
            },
        )));
    } else if !empty_session {
        body = body.child(row().child(ui::button(
            "Reset filters",
            "reset-filters",
            true,
            k,
            State::reset_filters,
        )));
    }
    body
}
fn footer(s: &State, k: &Kit) -> El {
    row()
        .h(26.0)
        .px(14.0)
        .gap(14.0)
        .center_y()
        .child(ui::badge(
            if s.workspace.session_name.is_some() {
                if s.workspace
                    .cache
                    .values()
                    .any(|e| e.request.json.is_some() || e.response.json.is_some())
                {
                    "SAVED WITH BODIES"
                } else {
                    "SAVED METADATA"
                }
            } else if s.demo {
                "SYNTHETIC DATA"
            } else if !s.connected {
                "CAPTURE UNKNOWN"
            } else if s.snapshot.capture_bodies {
                "CAPTURE ON"
            } else {
                "METADATA ONLY"
            },
            if s.workspace.session_name.is_some() || s.demo {
                k.c.text_muted
            } else if !s.connected {
                k.c.warning.fg
            } else if s.snapshot.capture_bodies {
                k.c.success.fg
            } else {
                k.c.text_muted
            },
            k,
        ))
        .child(
            label(
                if s.workspace.session_name.is_some() {
                    let total = s.snapshot.requests.len();
                    let with_json = s
                        .snapshot
                        .requests
                        .iter()
                        .filter(|r| {
                            s.workspace.cache.get(&r.id).is_some_and(|e| {
                                e.request.json.is_some() || e.response.json.is_some()
                            })
                        })
                        .count();
                    format!("Offline · no requests sent · {with_json}/{total} with JSON")
                } else if s.showcase.is_some() {
                    "Synthetic examples · no API calls".into()
                } else if s.demo {
                    "Synthetic fixture · no inference calls".into()
                } else {
                    clip(&s.snapshot.relay_url, 70)
                },
                11.0,
                k.c.text_muted,
            )
            .grow(1.0),
        )
        .child(label(
            format!(
                "{} evicted · {} capture drops",
                s.snapshot.evicted, s.snapshot.capture_dropped
            ),
            11.0,
            if s.snapshot.capture_dropped > 0 {
                k.c.warning.fg
            } else {
                k.c.text_faint
            },
        ))
        .child(label("F1  Help", 11.0, k.c.text_faint))
}
fn dialogs(s: &State, k: &Kit, layers: &mut Vec<El>) {
    let mut help = col().gap(13.0).padding(20.0);
    if s.tab == 3 {
        help = help.child(label("Diagram controls", 14.0, k.c.text_base))
            .child(label("Drag the background to pan. Wheel or pinch to zoom. Tab enters cards; Up and Down browse them. Canvas arrow keys pan, + and - zoom, 0 fits the graph, and 1 restores actual size. Fit graph shows relationships; Actual size makes cards readable. Focus a card's properties to scroll them with Page Up, Page Down, Home, or End.", 12.0, k.c.text_muted).wrap())
            .child(ui::rule(k));
    }
    for (title, body) in [
        (
            "Find an exchange",
            "Words combine with AND. Use quotes for phrases and - to exclude. Search checks request metadata; use Find in the JSON tabs to search bodies.",
        ),
        (
            "Search examples",
            "status:4xx   model:jev   method:POST\npath:systemone   request:abc   id:12\nduration:>200ms   duration:>=1s   -status:200",
        ),
        (
            "Keyboard",
            "Ctrl/Cmd+K   Search requests\nCtrl/Cmd+F   Find in the active JSON tab\nCtrl/Cmd+1–5   Switch detail tabs\nAlt+Up / Down   Previous / next request\nCtrl/Cmd+P   Pause / resume list\nCtrl/Cmd+B   Star selected exchange\nCtrl/Cmd+L   Clear view (Restore undoes it)\nCtrl/Cmd+E / O   Export / open session\nF1   Help     Escape   Close dialogs",
        ),
        (
            "Live traffic and retention",
            "Pause freezes the list; capture continues. Details may finish updating. Follow selects the newest matching request. Stars last while requests are retained; save a session to keep them.",
        ),
        (
            "What gets saved",
            "Sessions contain visible summaries and available inspection copies, after capture limits and redaction. No authentication headers are stored. Bodies may contain private application data. Export only when you want to save those captures.",
        ),
    ] {
        help = help
            .child(label(title, 13.0, k.c.text_base))
            .child(label(body, 12.0, k.c.text_muted).wrap());
    }
    if s.workspace.setup_help {
        help = col().padding(20.0).child(setup_view::guide(s, k));
    }
    Dialog::new(
        "help",
        if s.workspace.setup_help {
            "Connection setup"
        } else {
            "Help and shortcuts"
        },
    )
    .description("Search, inspect, compare, and save requests")
    .open(s.workspace.help)
    .width(600.0)
    .fit()
    .on_close(|s: &mut State| s.workspace.help = false)
    .child(
        row()
            .px(20.0)
            .gap(8.0)
            .child(ui::button(
                "Shortcuts",
                "help-shortcuts",
                !s.workspace.setup_help,
                k,
                |s| s.workspace.setup_help = false,
            ))
            .child(ui::button(
                "Connection setup",
                "help-setup",
                s.workspace.setup_help,
                k,
                |s| s.workspace.setup_help = true,
            )),
    )
    .child(
        rune_kit::scroll_bar(
            k,
            "help-scroll",
            "viewport",
            col()
                .h_full()
                .min_h(0.0)
                .focusable(true)
                .focus_ring(1.0, -1.0, k.c.text_faint)
                .role("region")
                .label("Help content")
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(help),
        )
        .h((s.workspace.window_height - 220.0).clamp(300.0, 560.0)),
    )
    .child(if s.workspace.window_height < 800.0 {
        row().px(20.0).py(8.0).fill(k.c.layer_02).child(label(
            "Scroll help · Page Down / End",
            11.0,
            k.c.text_muted,
        ))
    } else {
        el()
    })
    .build(k, layers);
    let busy = s.workspace.job.is_some();
    let mut choices = col().padding(20.0).gap(12.0);
    for (kind, title, note) in [
        (
            0,
            "Save session",
            "Visible requests + available captured bodies + stars. Reopen later in this app.",
        ),
        (
            1,
            "Export CSV",
            "Visible request summaries, timing, sizes, and status. No bodies.",
        ),
        (
            2,
            "Export exchange JSON",
            "The selected exchange and both inspection copies.",
        ),
    ] {
        choices = choices
            .child(
                Button::new(title)
                    .control_size(ControlSize::ExtraLarge)
                    .variant(ButtonVariant::Outline)
                    .disabled(
                        busy || if kind == 2 {
                            s.detail.is_none()
                        } else {
                            s.visible.is_empty()
                        },
                    )
                    .on_press_tx(move |s: &mut State, tx| {
                        s.workspace.export = false;
                        session::save_dialog(s, tx, kind);
                    })
                    .build(k)
                    .key(format!("export-{kind}")),
            )
            .child(label(note, 12.0, k.c.text_muted).wrap());
    }
    choices = choices.child(label("Body exports contain captured application data. Capture limits and redaction still apply.", 11.0, k.c.text_faint).wrap());
    Dialog::new("export", "Take your investigation with you")
        .description(format!(
            "{} visible requests · exports run in the background",
            s.visible.len()
        ))
        .open(s.workspace.export)
        .width(520.0)
        .fit()
        .on_close(|s: &mut State| s.workspace.export = false)
        .child(choices)
        .build(k, layers);
}
