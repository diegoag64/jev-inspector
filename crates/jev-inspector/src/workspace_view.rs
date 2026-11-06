use super::*;
use rune_kit::{Cell, ControlSize, Dialog, Table, TableColumn, TableStyle, TextInput};
use workspace::Filter;

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let k = ui::kit(s.dark);
    let header = row()
        .h(if s.workspace.window_height < 800.0 {
            52.0
        } else {
            64.0
        })
        .px(22.0)
        .gap(12.0)
        .center_y()
        .child(
            row()
                .size(34.0, 34.0)
                .center()
                .radius(9.0)
                .fill(k.alpha(k.c.text_accent, 0.12))
                .child(label("j", 26.0, k.c.text_accent)),
        )
        .child(label("jev", 23.0, k.c.text_base))
        .child(label("/", 21.0, k.c.text_faint))
        .child(label("inspector", 17.0, k.c.text_muted))
        .child(el().grow(1.0))
        .child(ui::badge(
            if s.workspace.session_name.is_some() {
                "SAVED SESSION"
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
            ui::action(&icons::OPEN, "Open session", false)
                .help("Open a saved session")
                .shortcut("cmd+o")
                .on_press_tx(|s: &mut State, tx| session::open_dialog(s, tx))
                .build(&k)
                .key("open-session"),
        )
        .child(
            ui::action(&icons::EXPORT, "Export", s.workspace.export)
                .help("Save session, exchange JSON, or CSV")
                .shortcut("cmd+e")
                .on_press(|s: &mut State| s.workspace.export = true)
                .build(&k)
                .key("export-menu"),
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
                    label(format!("Offline session · {name}"), 12.0, k.c.text_accent)
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
    let mut body = row().grow(1.0).min_h(0.0).padding(16.0).gap(5.0);
    if !expanded {
        body = body.child(traffic(s, &k, cx)).child(splitter(&k));
    }
    body = body.child(
        detail_view::detail_view(s, &k)
            .grow(1.0)
            .min_w(0.0)
            .h_full()
            .radius(10.0)
            .border(1.0, k.c.border_base)
            .fill(k.c.layer_01),
    );
    root = root.child(body).child(footer(s, &k));
    let mut layers = vec![];
    dialogs(s, &k, &mut layers);
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
    let compact = s.workspace.window_height < 800.0;
    let mut bar = row()
        .px(22.0)
        .py(if compact { 10.0 } else { 16.0 })
        .gap(22.0);
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
            "IN FLIGHT",
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
        if compact {
            bar = bar.child(
                row()
                    .grow(1.0)
                    .gap(8.0)
                    .center_y()
                    .child(label(title, 10.0, k.c.text_faint))
                    .child(label(value, 15.0, color))
                    .help(format!("{title}: {sub}")),
            );
        } else {
            bar = bar.child(
                col()
                    .grow(1.0)
                    .gap(6.0)
                    .child(label(title, 10.0, k.c.text_faint))
                    .child(label(value, 23.0, color))
                    .child(label(sub, 10.0, k.c.text_faint)),
            );
        }
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
        .radius(10.0)
        .clip_rounded(10.0)
        .border(1.0, k.c.border_base)
        .fill(k.c.layer_01)
        .child(
            row()
                .px(14.0)
                .py(12.0)
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
                    .control_size(ControlSize::ExtraLarge)
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
                        .control_size(ControlSize::ExtraLarge)
                        .variant(ButtonVariant::Ghost)
                        .help("Hide current rows in this window; restore them at any time")
                        .shortcut("cmd+l")
                        .on_press(State::clear_view)
                        .build(k)
                        .key("clear-view"),
                ),
        )
        .child(
            col().w_full().px(14.0).child(
                TextInput::new(&s.filter)
                    .width(None)
                    .placeholder("Search requests…  status:4xx  duration:>200")
                    .leading_icon(k.glyph.search)
                    .label("Search request metadata")
                    .clear_button()
                    .height(34.0)
                    .invalid(s.workspace.query_error.is_some())
                    .on_edit(|s: &mut State| &mut s.filter)
                    .on_change(|s: &mut State, _| s.filter_rows())
                    .on_key(workspace::on_key)
                    .build(k)
                    .key("request-filter"),
            ),
        );
    if let Some(error) = &s.workspace.query_error {
        panel = panel.child(label(error, 11.0, k.c.danger.fg).wrap().px(14.0).py(8.0));
    }
    panel = panel.child(
        row()
            .px(10.0)
            .py(10.0)
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
                "In flight",
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
            .px(14.0)
            .py(8.0)
            .fill(k.c.layer_02),
        );
    }
    if s.workspace.clear_before > 0 && !s.visible.is_empty() {
        panel = panel.child(
            row()
                .px(14.0)
                .py(5.0)
                .center_y()
                .child(label("Earlier requests hidden", 11.0, k.c.text_muted).grow(1.0))
                .child(ui::button("Restore", "restore-view", false, k, |s| {
                    s.workspace.clear_before = 0;
                    s.filter_rows();
                })),
        );
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
                .py(8.0)
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
    style.row_height = if s.workspace.compact { 32.0 } else { 54.0 };
    style.header_height = 33.0;
    style.cell_pad = 9.0;
    style.rule = k.c.border_base;
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
        TableColumn::new("time", "Elapsed")
            .width(82.0)
            .min(70.0)
            .sortable(),
    )
    .column(
        TableColumn::new("started", "Time · UTC")
            .width(105.0)
            .min(85.0)
            .sortable(),
    )
    .cell(move |s, i, column| {
        let r = &s.visible[i];
        let k = &table_kit;
        match column {
            "star" => Cell::text(if s.workspace.starred.contains(&r.id) {
                "★"
            } else {
                "·"
            })
            .color(if s.workspace.starred.contains(&r.id) {
                k.c.text_accent
            } else {
                k.c.text_faint
            }),
            "request" if !s.workspace.compact => Cell::el(
                col()
                    .gap(4.0)
                    .child(label(
                        format!("{}  {}", r.method, clip(&r.path, 24)),
                        12.0,
                        k.c.text_base,
                    ))
                    .child(label(
                        format!(
                            "#{:03} · {}",
                            r.id,
                            clip(r.model.as_deref().unwrap_or("Model unavailable"), 23)
                        ),
                        10.0,
                        k.c.text_muted,
                    )),
            ),
            "request" => Cell::text(format!("#{:03} {}", r.id, r.path)),
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
        .h(32.0)
        .px(22.0)
        .gap(18.0)
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
                    "Offline · no requests sent".into()
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
    Dialog::new("help", "Help and shortcuts")
        .description("Search, inspect, compare, and save requests")
        .open(s.workspace.help)
        .width(600.0)
        .fit()
        .on_close(|s: &mut State| s.workspace.help = false)
        .child(
            rune_kit::scroll_bar(
                k,
                "help-scroll",
                "viewport",
                col()
                    .h_full()
                    .min_h(0.0)
                    .focusable(true)
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
