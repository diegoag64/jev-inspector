use super::*;

pub fn tab_name(tab: u8) -> &'static str {
    match tab {
        1 => "Request",
        2 => "Response",
        3 => "Diagram",
        4 => "Compare",
        _ => "Overview",
    }
}

fn panel(s: &State) -> El {
    col()
        .key("detail-state")
        .role("tabpanel")
        .label(tab_name(s.tab))
        .grow(1.0)
        .min_h(0.0)
        .gap(16.0)
}

fn line(width: f32, height: f32, k: &Kit) -> El {
    el().w(width)
        .max_w(width)
        .h(height)
        .radius(4.0)
        .fill(k.c.border_base)
}

fn surface(k: &Kit) -> El {
    col()
        .w_full()
        .gap(16.0)
        .padding(18.0)
        .radius(10.0)
        .fill(k.c.bg_deep)
        .border(1.0, k.c.border_base)
}

fn progress(title: &str, note: &str, k: &Kit) -> El {
    row()
        .gap(12.0)
        .center_y()
        .child(rune_kit::Loader::new().size(18.0).build(k).a11y_hidden())
        .child(
            col()
                .gap(4.0)
                .child(
                    label(title, 13.0, k.c.text_base)
                        .role("status")
                        .key("detail-loading-status"),
                )
                .child(label(note, 12.0, k.c.text_muted).wrap()),
        )
}

pub fn missing(s: &State, k: &Kit) -> El {
    if s.workspace.session_name.is_some() {
        return message(
            s,
            k,
            "Summary only",
            "This session does not include the captured bodies for this request.",
            "Open Overview for the recorded timestamp, model, sizes, timing and request ID.",
            false,
        );
    }
    if let Some(error) = &s.connection_error {
        return message(
            s,
            k,
            "Relay unavailable",
            error,
            "Reconnecting automatically. Request details will load when the relay is available.",
            false,
        );
    }
    if let Some(error) = &s.detail_error {
        return message(
            s,
            k,
            "Couldn't load request details",
            error,
            "Retry loads the inspector's saved capture. It does not resend the API request.",
            true,
        );
    }
    loading(s, k)
}

fn message(s: &State, k: &Kit, title: &str, description: &str, note: &str, retry: bool) -> El {
    let mut content = surface(k)
        .key("detail-unavailable")
        .child(
            row()
                .gap(12.0)
                .center_y()
                .child(
                    el().size(32.0, 32.0)
                        .radius(8.0)
                        .fill(k.c.layer_02)
                        .child(rune::icon(&rune_kit::glyphs::FILE_TEXT, 18.0).color(k.c.text_muted))
                        .center(),
                )
                .child(
                    label(title, 16.0, k.c.text_base)
                        .wrap()
                        .grow(1.0)
                        .min_w(0.0),
                ),
        )
        .child(
            label(description, 13.0, k.c.text_muted)
                .wrap_anywhere()
                .role("status"),
        )
        .child(label(note, 12.0, k.c.text_muted).wrap());
    if retry {
        content =
            content.child(
                row().child(ui::button("Retry loading", "retry-detail", true, k, |s| {
                    s.detail_error = None;
                    s.mailbox.lock().expect("inspector mailbox").refresh = true;
                })),
            );
    }
    panel(s).child(content)
}

pub fn loading(s: &State, k: &Kit) -> El {
    let mut skeleton = col().gap(16.0).w_full().a11y_hidden();
    match s.tab {
        1 | 2 => {
            let mut code = surface(k)
                .gap(18.0)
                .child(
                    row()
                        .gap(12.0)
                        .child(line(32.0, 10.0, k))
                        .child(line(74.0, 10.0, k)),
                )
                .child(ui::rule(k));
            for (i, width) in [42.0, 180.0, 236.0, 144.0, 208.0, 168.0, 112.0, 42.0]
                .into_iter()
                .enumerate()
            {
                code = code.child(
                    row().gap(16.0).child(line(10.0, 8.0, k)).child(
                        row()
                            .pl(if i == 0 || i == 7 { 0.0 } else { 18.0 })
                            .child(line(width, 8.0, k)),
                    ),
                );
            }
            skeleton = skeleton.child(code);
        }
        3 => {
            skeleton = skeleton.child(
                surface(k).gap(22.0).child(line(100.0, 10.0, k)).child(
                    row()
                        .gap(16.0)
                        .child(
                            surface(k)
                                .grow(1.0)
                                .min_w(0.0)
                                .child(line(64.0, 10.0, k))
                                .child(line(100.0, 8.0, k)),
                        )
                        .child(
                            surface(k)
                                .grow(1.0)
                                .min_w(0.0)
                                .child(line(64.0, 10.0, k))
                                .child(line(100.0, 8.0, k)),
                        ),
                ),
            );
        }
        4 => {
            for _ in 0..2 {
                skeleton = skeleton.child(
                    surface(k)
                        .child(line(100.0, 10.0, k))
                        .child(ui::rule(k))
                        .child(line(220.0, 8.0, k))
                        .child(line(170.0, 8.0, k)),
                );
            }
        }
        _ => {
            let mut tiles = row().gap(12.0);
            for _ in 0..3 {
                tiles = tiles.child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .padding(16.0)
                        .gap(16.0)
                        .radius(8.0)
                        .fill(k.c.layer_02)
                        .child(line(54.0, 8.0, k))
                        .child(line(76.0, 20.0, k)),
                );
            }
            skeleton = skeleton
                .child(tiles)
                .child(
                    surface(k)
                        .child(line(110.0, 10.0, k))
                        .child(el().w_full().h(7.0).radius(4.0).fill(k.c.layer_02))
                        .child(
                            row()
                                .gap(24.0)
                                .child(line(110.0, 8.0, k))
                                .child(line(90.0, 8.0, k)),
                        ),
                )
                .child(
                    surface(k)
                        .child(line(90.0, 10.0, k))
                        .child(line(220.0, 8.0, k))
                        .child(line(160.0, 8.0, k)),
                );
        }
    }
    panel(s)
        .key("detail-loading")
        .child(progress(
            "Loading request details",
            "Fetching the captured exchange from the local relay.",
            k,
        ))
        .child(
            col()
                .key("loading-preview")
                .looping(rune::LoopSpec::opacity(1.8, 0.0, 0.65, 1.0))
                .a11y_hidden()
                .grow(1.0)
                .min_h(0.0)
                .clip_soft(0.0)
                .child(skeleton),
        )
}

pub fn waiting_response(s: &State, k: &Kit) -> El {
    if s.workspace.session_name.is_some() {
        return panel(s).key("response-incomplete-saved").child(surface(k)
            .child(label("Incomplete when saved", 16.0, k.c.text_base).role("status"))
            .child(label("This session was saved before the response completed. It is a historical snapshot and will not update. Request metadata and any captured request JSON remain available.", 13.0, k.c.text_muted).wrap()));
    }
    if s.connection_error.is_some() || (!s.connected && !s.demo) {
        return panel(s).key("response-pending-disconnected").child(surface(k)
            .child(label("Response status unknown", 16.0, k.c.text_base).role("status"))
            .child(label("The response was pending when the relay disconnected. Its current status cannot be checked until the connection returns.", 13.0, k.c.text_muted).wrap()));
    }
    panel(s).key("response-pending").child(surface(k)
        .child(progress("Waiting for response", "The upstream request is still in flight.", k))
        .child(ui::rule(k))
        .child(label("The response body is not complete yet. You can inspect the request while you wait.", 13.0, k.c.text_muted).wrap()))
}

pub fn empty_body(s: &State, k: &Kit) -> El {
    panel(s).key("body-empty").child(surface(k)
        .child(label("Empty response body", 16.0, k.c.text_base))
        .child(label("The upstream returned no response body. Status and timing are available in Overview.", 13.0, k.c.text_muted).wrap()))
}
