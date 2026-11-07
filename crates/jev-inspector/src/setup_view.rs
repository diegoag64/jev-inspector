use super::*;

fn command(title: &str, value: String, key: &'static str, k: &Kit) -> El {
    let copied = value.clone();
    col()
        .gap(6.0)
        .padding(12.0)
        .radius(8.0)
        .fill(k.c.bg_deep)
        .border(1.0, k.c.border_base)
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

pub fn guide(s: &State, k: &Kit) -> El {
    let relay = if s.snapshot.relay_url.starts_with("http") {
        s.snapshot.relay_url.trim_end_matches('/')
    } else {
        "http://127.0.0.1:8787"
    };
    let connection = if s.connection_file.is_absolute() {
        s.connection_file.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&s.connection_file)
    };
    let route = if cfg!(windows) {
        format!("$env:TYPESAFE_BASE_URL = \"{relay}\"")
    } else {
        format!("export TYPESAFE_BASE_URL=\"{relay}\"")
    };
    col().w_full().gap(12.0)
        .child(label(if s.connected { "Relay connected — route your backend" } else { "Connect your backend" }, 20.0, k.c.text_base).wrap())
        .child(label("Run these commands from this repository. Your backend keeps its existing API key and makes requests when you choose.", 13.0, k.c.text_muted).wrap())
        .child(command("1. Start the relay in a terminal", "cargo run -p jev-bridge -- --capture-bodies".into(), "setup-relay", k))
        .child(label("Body capture enables JSON, answers, and diagrams. Omit --capture-bodies for metadata only.", 12.0, k.c.text_muted).wrap())
        .child(command("2. Open the inspector in another terminal", format!("cargo run -p jev-inspector -- --connection-file \"{}\"", connection.display()), "setup-inspector", k))
        .child(command("3. Set the URL in your backend's terminal", route, "setup-route", k))
        .child(label("Then start your backend with its usual command. If your SDK configures baseUrl or baseURL explicitly, set that option to this relay origin instead. Do not append /v1.", 12.0, k.c.text_muted).wrap())
        .child(label(format!("Relay: {relay}\nConnection file: {}", connection.display()), 12.0, k.c.text_muted).wrap_anywhere().selectable(true))
        .child(label("Send one normal request from your backend. It should appear in Traffic; select it to inspect the exchange. Additional setup and SDK examples are in docs/bridge-quickstart.md.", 12.0, k.c.text_muted).wrap())
}

pub fn panel(s: &State, k: &Kit, cx: &Cx<State>) -> El {
    request_scroll::area(
        cx,
        k,
        "setup-scroll",
        "setup-content",
        col()
            .key("setup-scroll")
            .grow(1.0)
            .min_h(0.0)
            .padding(20.0)
            .scroll(rune::ScrollSpec {
                stick_bottom: false,
            })
            .focusable(true)
            .role("region")
            .label("Connection setup")
            .child(guide(s, k).key("setup-content")),
        false,
    )
}
