use super::*;

fn command(title: &str, value: String, key: &'static str, k: &Kit) -> El {
    let copied = value.clone();
    ui::surface(col(), ui::CARD_RADIUS, k.c.bg_deep)
        .gap(6.0)
        .padding(12.0)
        .child(
            row()
                .center_y()
                .gap(8.0)
                .child(label(title, 12.0, k.c.text_base).grow(1.0))
                .child(
                    ui::action(&rune_kit::glyphs::COPY, "Copy command", false)
                        .on_press_tx(move |s: &mut State, tx| {
                            tx.copy(copied.clone());
                            s.workspace.notice = Some(("Setup command copied".into(), false));
                        })
                        .build(k)
                        .key(key),
                ),
        )
        .child(
            ui::mono(value, 12.0, k.c.text_base)
                .wrap_anywhere()
                .selectable(true),
        )
}

fn quote(value: &str, windows: bool) -> String {
    if windows {
        format!("'{}'", value.replace('\'', "''"))
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

pub(super) fn commands(s: &State) -> (String, String, String, String) {
    let listen = s
        .snapshot
        .relay_url
        .strip_prefix("http://")
        .and_then(|url| {
            url.trim_end_matches('/')
                .parse::<std::net::SocketAddr>()
                .ok()
        })
        .filter(|addr| addr.ip().is_loopback() && addr.port() != 0)
        .unwrap_or_else(|| "127.0.0.1:8787".parse().unwrap());
    let relay = format!("http://{listen}");
    let connection = if s.connection_file.is_absolute() {
        s.connection_file.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&s.connection_file)
    };
    let windows = cfg!(windows);
    let path = quote(&connection.to_string_lossy(), windows);
    let custom_path = s.connection_file != std::path::Path::new(".jev/connection.json");
    let custom_listen = listen.to_string() != "127.0.0.1:8787";
    let mut bridge = "cargo run -p jev-bridge -- --capture-bodies".to_owned();
    if s.snapshot.history_limit > 0 && s.snapshot.history_limit != 100 {
        bridge.push_str(&format!(" --history {}", s.snapshot.history_limit));
    }
    if let Ok(upstream) = reqwest::Url::parse(&s.snapshot.upstream)
        && matches!(upstream.scheme(), "http" | "https")
        && upstream.host_str().is_some()
        && upstream.username().is_empty()
        && upstream.password().is_none()
        && upstream.as_str().trim_end_matches('/') != "https://api.typesafe.ai"
    {
        bridge.push_str(&format!(
            " --upstream {}",
            quote(upstream.as_str(), windows)
        ));
    }
    if custom_path {
        bridge.push_str(&format!(" --connection-file {path}"));
    }
    if custom_listen {
        bridge.push_str(&format!(
            " --listen {}",
            quote(&listen.to_string(), windows)
        ));
    }
    if custom_path || custom_listen {
        // An ephemeral inspection port avoids collisions with an existing relay.
        bridge.push_str(" --inspector 127.0.0.1:0");
    }
    let inspector = format!("cargo run -p jev-inspector -- --connection-file {path}");
    let route = if windows {
        format!("$env:TYPESAFE_BASE_URL = {}", quote(&relay, true))
    } else {
        format!("export TYPESAFE_BASE_URL={}", quote(&relay, false))
    };
    (bridge, inspector, route, relay)
}

pub fn guide(s: &State, k: &Kit) -> El {
    let (bridge, inspector, route, relay) = commands(s);
    let mut guide = col().w_full().gap(12.0)
        .child(label(if s.connected { "Route your backend" } else { "Connect your backend" }, 20.0, k.c.text_base).wrap())
        .child(label(if s.connected { "The relay is connected. Keep it running and send a request from your backend." } else { "Run these commands from this repository. Your backend keeps its existing API key." }, 13.0, k.c.text_muted).wrap());
    let routing = label("Start your backend. If it sets baseUrl, baseURL, or Python base_url explicitly, use this relay URL instead. Do not append /v1.", 12.0, k.c.text_muted).wrap();
    let send = label(
        "Send a request, then select it in Traffic to inspect it.",
        12.0,
        k.c.text_muted,
    )
    .wrap();
    if s.connected {
        guide = guide
            .child(command(
                "1. Set the URL in your backend's terminal",
                route,
                "setup-route",
                k,
            ))
            .child(routing)
            .child(label("2. Send a request", 13.0, k.c.text_base))
            .child(send)
            .child(ui::rule(k))
            .child(label("Connection tools", 14.0, k.c.text_base))
            .child(command(
                "Inspector connection command",
                inspector,
                "setup-inspector",
                k,
            ))
            .child(
                command("Relay restart command", bridge, "setup-relay", k).child(
                    label(
                        "Stop the existing relay before running this restart command. It preserves the upstream, listener, and history limit; reapply any custom timeout before restarting.",
                        12.0,
                        k.c.text_muted,
                    )
                    .wrap(),
                ),
            );
    } else {
        guide = guide
            .child(command(
                "1. Start the relay in a terminal",
                bridge,
                "setup-relay",
                k,
            ))
            .child(command(
                "2. Inspector connection command",
                inspector,
                "setup-inspector",
                k,
            ))
            .child(command(
                "3. Set the URL in your backend's terminal",
                route,
                "setup-route",
                k,
            ))
            .child(routing)
            .child(send);
    }
    guide.child(label("Body capture enables JSON, answers, and diagrams. Omit --capture-bodies for metadata only.", 12.0, k.c.text_muted).wrap())
        .child(label(format!("Relay: {relay}\nConnection file: {}", s.connection_file.display()), 12.0, k.c.text_muted).wrap_anywhere().selectable(true))
        .child(label("Additional setup and SDK examples are in docs/bridge-quickstart.md.", 12.0, k.c.text_muted).wrap())
}

pub fn panel(s: &State, k: &Kit, cx: &Cx<State>) -> El {
    col()
        .grow(1.0)
        .min_h(0.0)
        .padding(20.0)
        .child(request_scroll::area(
            cx,
            k,
            "setup-scroll",
            "setup-content",
            col()
                .key("setup-scroll")
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .focusable(true)
                .focus_ring(1.0, -1.0, k.c.text_faint)
                .role("region")
                .label("Connection setup")
                .child(guide(s, k).key("setup-content")),
            false,
        ))
}

#[cfg(test)]
mod tests {
    use super::quote;
    #[test]
    fn command_arguments_are_literal_in_both_supported_shells() {
        assert_eq!(
            quote("O'Brien $HOME `literal`", true),
            "'O''Brien $HOME `literal`'"
        );
        assert_eq!(
            quote("O'Brien $HOME `literal`", false),
            "'O'\"'\"'Brien $HOME `literal`'"
        );
    }
}
