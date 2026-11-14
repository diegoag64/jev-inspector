//! First-run setup for the desktop app: welcome, relay port, capture mode, and
//! routing the backend. Reopened later from Relay settings to change the port.
use super::*;
use crate::relay_host::{self, Settings};
use rune_kit::{ControlSize, Spinner, TextInput};

pub const WELCOME: u8 = 0;
pub const PORT: u8 = 1;
pub const CAPTURE: u8 = 2;
pub const CONNECT: u8 = 3;

#[derive(Clone, Debug, PartialEq)]
pub enum PortStatus {
    Available,
    /// The relay this app is already running listens here.
    Current,
    Busy(String, Option<u16>),
    Invalid(String),
}

pub struct Onboarding {
    /// Desktop app mode: this process hosts the relay (or previews doing so).
    pub app: bool,
    /// Render the flow without binding ports. Used by headless QA.
    pub preview: bool,
    pub open: bool,
    pub first_run: bool,
    pub step: u8,
    pub port: TextEdit,
    pub capture: bool,
    pub status: PortStatus,
    pub error: Option<String>,
    /// The committed configuration, saved in preferences.
    pub settings: Settings,
    /// Snapshot instance before the relay (re)started, so stale history does not
    /// count as the first request.
    pub previous_instance: Option<String>,
    pub copied: bool,
    /// The user stopped the hosted relay, or it could not start at launch.
    pub stopped: bool,
}

impl Default for Onboarding {
    fn default() -> Self {
        Self {
            app: false,
            preview: false,
            open: false,
            first_run: true,
            step: WELCOME,
            port: TextEdit::new(relay_host::DEFAULT_PORT.to_string()),
            capture: true,
            status: PortStatus::Available,
            error: None,
            settings: Settings::default(),
            previous_instance: None,
            copied: false,
            stopped: false,
        }
    }
}

fn pink() -> Color {
    rune::color::hex("#e551ba").unwrap()
}
fn violet() -> Color {
    rune::color::hex("#7c5cff").unwrap()
}

/// Open the flow at `step` with the committed settings as the draft.
pub fn open(s: &mut State, step: u8) {
    let ob = &mut s.onboarding;
    ob.open = true;
    ob.step = step;
    ob.port = TextEdit::new(ob.settings.port.to_string());
    ob.capture = ob.settings.capture_bodies;
    ob.error = None;
    ob.copied = false;
    refresh(s);
}

/// Revalidate the typed port.
pub fn refresh(s: &mut State) {
    let running = s.relay.as_ref().and_then(|h| h.running()).map(|r| r.port);
    let preview = s.onboarding.preview;
    s.onboarding.status = match relay_host::parse_port(&s.onboarding.port.text) {
        Err(message) => PortStatus::Invalid(message),
        Ok(port) if running == Some(port) => PortStatus::Current,
        Ok(_) if preview => PortStatus::Available,
        Ok(port) => match relay_host::check(port) {
            Ok(()) => PortStatus::Available,
            Err(message) => PortStatus::Busy(message, relay_host::suggest(port)),
        },
    };
}

fn port_ready(s: &State) -> bool {
    matches!(
        s.onboarding.status,
        PortStatus::Available | PortStatus::Current
    )
}

fn go(s: &mut State, step: u8) {
    s.onboarding.step = step;
    s.onboarding.error = None;
    if step == PORT {
        refresh(s);
    }
}

fn continue_from_port(s: &mut State, tx: &mut Tx) {
    refresh(s);
    if port_ready(s) {
        go(s, CAPTURE);
        tx.focus("onboarding-capture-full", false);
    }
}

/// Start (or restart) the relay with the draft settings and move to Connect.
fn start_relay(s: &mut State) {
    let Ok(port) = relay_host::parse_port(&s.onboarding.port.text) else {
        go(s, PORT);
        return;
    };
    let settings = Settings {
        port,
        capture_bodies: s.onboarding.capture,
    };
    let unchanged = s.relay.as_ref().and_then(|h| h.running()) == Some(settings);
    if !unchanged {
        s.onboarding.previous_instance = Some(s.snapshot.instance_id.clone());
        if let Some(host) = &s.relay
            && let Err(error) = host.start(settings)
        {
            go(s, PORT);
            s.onboarding.error = Some(error);
            return;
        }
        s.mailbox.lock().expect("inspector mailbox").refresh = true;
    }
    s.onboarding.settings = settings;
    if s.onboarding.stopped {
        s.onboarding.stopped = false;
        s.connection_error = None;
    }
    let prefs = &mut s.workspace.prefs;
    if let Err(e) = prefs
        .set("relay", &settings)
        .and_then(|()| prefs.set("onboarded", &true))
    {
        s.workspace.notice = Some((e.to_string(), true));
    }
    s.onboarding.copied = false;
    go(s, CONNECT);
}

pub fn stopped_message(s: &State) -> String {
    format!(
        "Relay stopped. Requests to {} fail until you start it again. Captured requests stay visible.",
        relay_host::relay_url(s.onboarding.settings.port)
    )
}

/// Stop the hosted relay. The workspace keeps the requests it already received.
pub fn stop_relay(s: &mut State) {
    if let Some(host) = &s.relay {
        host.stop();
    }
    s.onboarding.stopped = true;
    s.connected = false;
    s.connection_error = Some(stopped_message(s));
}

/// Start the hosted relay again with the saved settings. If the port was taken
/// in the meantime, open Relay settings with the error.
pub fn resume_relay(s: &mut State) {
    if let Some(host) = &s.relay
        && let Err(error) = host.start(s.onboarding.settings)
    {
        s.onboarding.first_run = false;
        open(s, PORT);
        s.onboarding.error = Some(format!("{error} Choose another port to start capturing."));
        return;
    }
    s.onboarding.stopped = false;
    s.connection_error = None;
    s.mailbox.lock().expect("inspector mailbox").refresh = true;
}

pub fn finish(s: &mut State) {
    s.onboarding.open = false;
    s.onboarding.first_run = false;
}

pub fn on_key(s: &mut State, event: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    if event.key == rune::Key::Escape && !s.onboarding.first_run {
        finish(s);
        return true;
    }
    false
}

/// The latest exchange from the relay started in this flow, if one arrived.
fn first_request(s: &State) -> Option<&Summary> {
    let fresh = s.onboarding.previous_instance.as_deref() != Some(s.snapshot.instance_id.as_str());
    fresh
        .then(|| s.snapshot.requests.iter().max_by_key(|r| r.id))
        .flatten()
}

pub fn route_command(relay: &str) -> String {
    if cfg!(windows) {
        format!("$env:TYPESAFE_BASE_URL = '{relay}'")
    } else {
        format!("export TYPESAFE_BASE_URL={relay}")
    }
}

// ---------------------------------------------------------------- view

/// Entrance for each block of a step: rise a few points and fade in, staggered.
fn rise(e: El, order: u8) -> El {
    e.transition(
        rune::Transition::asymmetric(
            rune::Transition::offset(0.0, 10.0).combined(rune::Transition::opacity()),
            rune::Transition::none(),
        )
        .springs(rune::SpringPair::new(
            rune::Spring::new(0.42, 0.0),
            rune::Spring::NONE,
        )),
    )
    .enter_delay(f32::from(order) * 0.045)
}

fn centered(value: impl Into<String>, size: f32, color: Color) -> El {
    label(value, size, color).wrap().align(rune::Align::Center)
}

/// The app mark: a source, the relay ring, and the upstream on a pink-to-violet tile.
pub fn mark(size: f32) -> El {
    let white = Color::WHITE;
    let stroke = (size * 0.05).max(1.5);
    let dot = |d: f32| el().w(d).h(d).radius(d / 2.0).fill(white);
    let wire = |w: f32| el().w(w).h(stroke).fill(white.alpha(0.8));
    let ring = size * 0.32;
    row()
        .w(size)
        .h(size)
        .center()
        .radius(size * 0.25)
        .continuous_corners()
        .fill(pink())
        .linear_gradient(
            std::f32::consts::PI * 0.75,
            [(0.0, pink()), (1.0, violet())],
        )
        .shadow(size * 0.14, size * 0.45, pink().alpha(0.35))
        .child(dot(size * 0.11))
        .child(wire(size * 0.09))
        .child(
            row()
                .w(ring)
                .h(ring)
                .center()
                .radius(ring / 2.0)
                .border(stroke, white)
                .child(dot(ring * 0.32)),
        )
        .child(wire(size * 0.09))
        .child(dot(size * 0.11))
        .role("image")
        .label("Jev Inspector")
}

fn primary(name: &str, disabled: bool) -> Button {
    Button::new(name)
        .variant(ButtonVariant::Contrast)
        .control_size(ControlSize::Large)
        .disabled(disabled)
}

fn secondary(name: &str) -> Button {
    Button::new(name)
        .variant(ButtonVariant::Ghost)
        .control_size(ControlSize::Large)
}

fn feature(icon: &'static rune::Icon, title: &str, body: &str, k: &Kit) -> El {
    ui::surface(col(), ui::CARD_RADIUS + 2.0, k.c.layer_01)
        .border(1.0, k.c.border_base)
        .w(0.0)
        .grow(1.0)
        .min_w(0.0)
        .padding(16.0)
        .gap(8.0)
        .child(
            row()
                .w(30.0)
                .h(30.0)
                .center()
                .radius(8.0)
                .continuous_corners()
                .fill(pink().alpha(if k.dark { 0.16 } else { 0.12 }))
                .child(rune::icon(icon, 17.0).color(accent(k))),
        )
        .child(ui::heading(title, 13.0, k.c.text_base))
        .child(label(body, 12.0, k.c.text_muted).wrap())
}

/// Brand pink for small marks, darkened in light mode to keep 3:1 against cards.
fn accent(k: &Kit) -> Color {
    rune::color::hex(if k.dark { "#f07cd0" } else { "#b8268c" }).unwrap()
}

fn eyebrow(step: u8, k: &Kit, first_run: bool) -> El {
    label(
        if first_run {
            format!("Step {step} of 3")
        } else {
            "Relay settings".into()
        },
        12.0,
        accent(k),
    )
    .font(rune::TextStyle::new(12.0, 600.0))
}

fn welcome(k: &Kit) -> El {
    col()
        .w_full()
        .center()
        .gap(0.0)
        .child(rise(mark(76.0), 0))
        .child(el().h(28.0))
        .child(rise(
            ui::heading("Welcome to Jev Inspector", 30.0, k.c.text_base)
                .align(rune::Align::Center)
                .key("onboarding-title"),
            1,
        ))
        .child(el().h(10.0))
        .child(rise(
            centered(
                "See every request your backend sends to TypeSafe and every answer Jev returns, live, on your Mac.",
                15.0,
                k.c.text_muted,
            )
            .max_w(460.0),
            2,
        ))
        .child(el().h(32.0))
        .child(rise(
            row()
                .w_full()
                .gap(12.0)
                .child(feature(
                    &icons::PULSE,
                    "Live traffic",
                    "Requests, status and timing as they happen.",
                    k,
                ))
                .child(feature(
                    &rune_kit::glyphs::SPARKLE,
                    "Answers",
                    "Returned choices with probabilities and confidence.",
                    k,
                ))
                .child(feature(
                    &icons::NODES,
                    "Diagrams",
                    "Each question drawn as connected cards.",
                    k,
                )),
            3,
        ))
        .child(el().h(36.0))
        .child(rise(
            primary("Get started", false)
                .on_press_tx(|s: &mut State, tx| {
                    go(s, PORT);
                    tx.focus("onboarding-port", false);
                })
                .build(k)
                .min_w(180.0)
                .key("onboarding-start"),
            4,
        ))
        .child(el().h(16.0))
        .child(rise(
            centered(
                "Everything runs locally. Captures stay in memory and clear when you quit.",
                12.0,
                k.c.text_faint,
            ),
            5,
        ))
}

fn flow_node(icon: &'static rune::Icon, title: &str, detail: El, highlight: bool, k: &Kit) -> El {
    let node = ui::surface(col(), ui::CARD_RADIUS, k.c.layer_01)
        .w(150.0)
        .padding(12.0)
        .gap(6.0)
        .center()
        .child(rune::icon(icon, 18.0).color(if highlight { accent(k) } else { k.c.text_muted }))
        .child(ui::heading(title, 12.0, k.c.text_base))
        .child(detail);
    if highlight {
        node.border(1.5, pink().alpha(0.7)).shadow(
            4.0,
            18.0,
            pink().alpha(if k.dark { 0.22 } else { 0.14 }),
        )
    } else {
        node.border(1.0, k.c.border_base)
    }
}

fn wire(k: &Kit) -> El {
    row()
        .grow(1.0)
        .min_w(16.0)
        .center_y()
        .child(el().grow(1.0).h(1.5).fill(k.c.border_base))
        .child(rune::icon(&rune_kit::glyphs::CARET_RIGHT, 12.0).color(k.c.text_faint))
}

fn port_step(s: &State, k: &Kit) -> El {
    let ob = &s.onboarding;
    let port = relay_host::parse_port(&ob.port.text).ok();
    let shown = port.map_or("—".to_owned(), |p| p.to_string());
    let (chip, chip_color) = match &ob.status {
        PortStatus::Available => ("Available", k.c.success.fg),
        PortStatus::Current => ("In use by this app", k.c.success.fg),
        PortStatus::Busy(..) => ("In use", k.c.danger.fg),
        PortStatus::Invalid(_) => ("Invalid", k.c.danger.fg),
    };
    let mut help = col().gap(8.0).min_h(44.0).w_full();
    let message = ob.error.clone().or(match &ob.status {
        PortStatus::Busy(message, _) | PortStatus::Invalid(message) => Some(message.clone()),
        _ => None,
    });
    if let Some(message) = message {
        let mut line = row().gap(10.0).center_y().child(
            label(message, 12.0, k.c.danger.fg)
                .wrap()
                .grow(1.0)
                .min_w(0.0),
        );
        if let PortStatus::Busy(_, Some(suggestion)) = ob.status {
            line = line.child(
                Button::new(format!("Use {suggestion}"))
                    .variant(ButtonVariant::Neutral)
                    .on_press(move |s: &mut State| {
                        s.onboarding.port = TextEdit::new(suggestion.to_string());
                        s.onboarding.error = None;
                        refresh(s);
                    })
                    .build(k)
                    .key("onboarding-suggest"),
            );
        }
        help = help.child(line);
    } else {
        help = help.child(
            label(
                "8787 works for most setups. Pick another if something else already uses it.",
                12.0,
                k.c.text_muted,
            )
            .wrap(),
        );
    }
    let field = TextInput::new(&ob.port)
        .label("Relay port")
        .placeholder("8787")
        .width(Some(132.0))
        .large()
        .text_style(rune::TextStyle::new(18.0, 500.0).mono())
        .invalid(matches!(
            ob.status,
            PortStatus::Busy(..) | PortStatus::Invalid(_)
        ))
        .on_edit(|s: &mut State| &mut s.onboarding.port)
        .on_change(|s: &mut State, _| {
            s.onboarding.error = None;
            refresh(s);
        })
        .on_submit(continue_from_port)
        .build(&ui::field_kit(k))
        .key("onboarding-port");
    let back = if ob.first_run {
        secondary("Back")
            .on_press(|s: &mut State| go(s, WELCOME))
            .build(k)
            .key("onboarding-back")
    } else {
        secondary("Cancel")
            .on_press(finish)
            .build(k)
            .key("onboarding-cancel")
    };
    col()
        .w_full()
        .gap(0.0)
        .child(rise(eyebrow(1, k, ob.first_run), 0))
        .child(el().h(8.0))
        .child(rise(
            ui::heading("Choose a port for the relay", 26.0, k.c.text_base)
                .wrap()
                .key("onboarding-title"),
            1,
        ))
        .child(el().h(10.0))
        .child(rise(
            label(
                "Your backend sends requests to this port instead of straight to TypeSafe. The relay forwards each one unchanged and keeps a copy for you to inspect.",
                14.0,
                k.c.text_muted,
            )
            .wrap(),
            2,
        ))
        .child(el().h(26.0))
        .child(rise(
            row()
                .w_full()
                .center_y()
                .child(flow_node(
                    &icons::SERVER,
                    "Your backend",
                    label("Existing API key", 11.0, k.c.text_muted),
                    false,
                    k,
                ))
                .child(wire(k))
                .child(flow_node(
                    &icons::RELAY,
                    "Jev relay",
                    ui::mono(format!("127.0.0.1:{shown}"), 11.0, accent(k))
                        .key("onboarding-flow-port"),
                    true,
                    k,
                ))
                .child(wire(k))
                .child(flow_node(
                    &icons::CLOUD,
                    "TypeSafe",
                    label("api.typesafe.ai", 11.0, k.c.text_muted),
                    false,
                    k,
                )),
            3,
        ))
        .child(el().h(28.0))
        .child(rise(
            col()
                .w_full()
                .gap(10.0)
                .child(label("Port", 12.0, k.c.text_muted).font(rune::TextStyle::new(12.0, 600.0)))
                .child(
                    row()
                        .gap(12.0)
                        .center_y()
                        .child(field)
                        .child(ui::badge(chip, chip_color, k).key("onboarding-port-status"))
                        .child(el().grow(1.0))
                        .child(
                            col()
                                .gap(2.0)
                                .child(label("Relay URL", 11.0, k.c.text_faint).align(rune::Align::End))
                                .child(
                                    ui::mono(relay_host::relay_url(port.unwrap_or(0)).replace(":0", ":—"), 13.0, k.c.text_base)
                                        .selectable(true)
                                        .align(rune::Align::End),
                                ),
                        ),
                )
                .child(help),
            4,
        ))
        .child(el().h(22.0))
        .child(rise(
            row()
                .w_full()
                .center_y()
                .child(back)
                .child(el().grow(1.0))
                .child(
                    primary("Continue", !port_ready(s))
                        .on_press_tx(continue_from_port)
                        .build(k)
                        .min_w(140.0)
                        .key("onboarding-continue"),
                ),
            5,
        ))
}

fn capture_option(
    key: &str,
    title: &str,
    tag: Option<&str>,
    body: &str,
    value: bool,
    s: &State,
    k: &Kit,
) -> El {
    let selected = s.onboarding.capture == value;
    let radio = row()
        .w(18.0)
        .h(18.0)
        .center()
        .radius(9.0)
        .border(
            if selected { 5.5 } else { 1.5 },
            if selected {
                k.c.text_base
            } else {
                k.c.text_faint
            },
        )
        .motion(rune::Spring::new(0.25, 0.0));
    let mut heading = row()
        .gap(8.0)
        .center_y()
        .child(ui::heading(title, 14.0, k.c.text_base));
    if let Some(tag) = tag {
        heading = heading.child(ui::badge(tag, accent(k), k));
    }
    ui::surface(row(), ui::CARD_RADIUS + 2.0, k.c.layer_01)
        .w_full()
        .padding(16.0)
        .gap(14.0)
        .border(
            if selected { 1.5 } else { 1.0 },
            if selected {
                k.c.text_base
            } else {
                k.c.border_base
            },
        )
        .motion(rune::Spring::new(0.25, 0.0))
        .hover(k.c.layer_02)
        .child(col().pt(1.0).child(radio))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(5.0)
                .child(heading)
                .child(label(body, 12.0, k.c.text_muted).wrap()),
        )
        .on_press(move |s: &mut State| s.onboarding.capture = value)
        .role("radio")
        .checked(selected)
        .label(title)
        .focus_ring(2.0, 2.0, k.c.text_faint)
        .key(key)
}

fn capture_step(s: &State, k: &Kit) -> El {
    let ob = &s.onboarding;
    col()
        .w_full()
        .gap(0.0)
        .child(rise(eyebrow(2, k, ob.first_run), 0))
        .child(el().h(8.0))
        .child(rise(
            ui::heading("What should the relay keep?", 26.0, k.c.text_base)
                .wrap()
                .key("onboarding-title"),
            1,
        ))
        .child(el().h(10.0))
        .child(rise(
            label(
                "Captures stay in memory on this Mac and clear when you quit. API keys and request headers are never stored.",
                14.0,
                k.c.text_muted,
            )
            .wrap(),
            2,
        ))
        .child(el().h(24.0))
        .child(rise(
            col()
                .w_full()
                .gap(10.0)
                .role("radiogroup")
                .label("Capture mode")
                .child(capture_option(
                    "onboarding-capture-full",
                    "Full capture",
                    Some("Recommended"),
                    "Request and response JSON, Jev answers, probabilities and diagrams. Common secret field names are redacted.",
                    true,
                    s,
                    k,
                ))
                .child(capture_option(
                    "onboarding-capture-metadata",
                    "Metadata only",
                    None,
                    "Method, path, status, timing, size and model. No request or response bodies.",
                    false,
                    s,
                    k,
                )),
            3,
        ))
        .child(el().h(16.0))
        .child(rise(
            row()
                .gap(8.0)
                .center_y()
                .child(rune::icon(&icons::SHIELD, 15.0).color(k.c.text_faint))
                .child(
                    label(
                        "Captured bodies can still contain application data. Review exports before sharing them.",
                        12.0,
                        k.c.text_faint,
                    )
                    .wrap()
                    .grow(1.0)
                    .min_w(0.0),
                ),
            4,
        ))
        .child(el().h(28.0))
        .child(rise(
            row()
                .w_full()
                .center_y()
                .child(
                    secondary("Back")
                        .on_press(|s: &mut State| go(s, PORT))
                        .build(k)
                        .key("onboarding-back"),
                )
                .child(el().grow(1.0))
                .child(
                    primary(if ob.first_run { "Start relay" } else { "Restart relay" }, false)
                        .on_press(start_relay)
                        .build(k)
                        .min_w(140.0)
                        .key("onboarding-start-relay"),
                ),
            5,
        ))
}

fn connect_step(s: &State, k: &Kit) -> El {
    let ob = &s.onboarding;
    let relay = relay_host::relay_url(ob.settings.port);
    let command = route_command(&relay);
    let copied = command.clone();
    let received = first_request(s);
    let running = row()
        .gap(8.0)
        .center_y()
        .px(10.0)
        .py(5.0)
        .radius(12.0)
        .fill(k.alpha(k.c.success.fg, if k.dark { 0.14 } else { 0.1 }))
        .child(el().w(7.0).h(7.0).radius(3.5).fill(k.c.success.fg))
        .child(
            label(
                format!(
                    "Relay running on 127.0.0.1:{} · {}",
                    ob.settings.port,
                    if ob.settings.capture_bodies {
                        "full capture"
                    } else {
                        "metadata only"
                    }
                ),
                12.0,
                k.c.success.fg,
            )
            .font(rune::TextStyle::new(12.0, 590.0)),
        )
        .key("onboarding-relay-status");
    let command_card = ui::surface(row(), ui::CARD_RADIUS + 2.0, k.c.bg_deep)
        .w_full()
        .border(1.0, k.c.border_base)
        .pl(16.0)
        .pr(8.0)
        .py(8.0)
        .gap(10.0)
        .center_y()
        .child(label("$", 13.0, k.c.text_faint).font(rune::TextStyle::new(13.0, 400.0).mono()))
        .child(
            ui::mono(command, 13.0, k.c.text_base)
                .selectable(true)
                .wrap_anywhere()
                .grow(1.0)
                .min_w(0.0),
        )
        .child(
            Button::new(if ob.copied { "Copied" } else { "Copy" })
                .icon(if ob.copied {
                    &rune_kit::glyphs::CHECK
                } else {
                    &rune_kit::glyphs::COPY
                })
                .variant(ButtonVariant::Neutral)
                .on_press_tx(move |s: &mut State, tx| {
                    tx.copy(copied.clone());
                    s.onboarding.copied = true;
                })
                .build(k)
                .key("onboarding-copy"),
        );
    let listening = match received {
        None => ui::surface(row(), ui::CARD_RADIUS + 2.0, k.c.layer_01)
            .w_full()
            .border(1.0, k.c.border_base)
            .padding(16.0)
            .gap(14.0)
            .center_y()
            .child(Spinner::new().size(16.0).color(k.c.text_muted).build(k))
            .child(
                col()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(3.0)
                    .child(ui::heading(
                        "Waiting for your first request…",
                        13.0,
                        k.c.text_base,
                    ))
                    .child(
                        label(
                            "Requests appear in the inspector as soon as your backend sends one.",
                            12.0,
                            k.c.text_muted,
                        )
                        .wrap(),
                    ),
            )
            .key("onboarding-waiting"),
        Some(r) => ui::surface(row(), ui::CARD_RADIUS + 2.0, k.c.layer_01)
            .w_full()
            .border(1.0, k.alpha(k.c.success.fg, 0.5))
            .padding(16.0)
            .gap(14.0)
            .center_y()
            .child(
                row()
                    .w(22.0)
                    .h(22.0)
                    .center()
                    .radius(11.0)
                    .fill(k.c.success.fg)
                    .child(rune::icon(&rune_kit::glyphs::CHECK, 14.0).color(k.c.text_contrast)),
            )
            .child(
                col()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(3.0)
                    .child(ui::heading("First request received", 13.0, k.c.text_base))
                    .child(
                        ui::mono(
                            format!(
                                "{} {} · {} · {}",
                                r.method,
                                r.path,
                                status(r),
                                ui::duration(r.duration_ms)
                            ),
                            12.0,
                            k.c.text_muted,
                        )
                        .truncate_middle(),
                    ),
            )
            .transition(rune::Transition::opacity())
            .key("onboarding-received"),
    };
    let back = if ob.first_run {
        secondary("Back")
            .on_press(|s: &mut State| go(s, CAPTURE))
            .build(k)
            .key("onboarding-back")
    } else {
        el()
    };
    col()
        .w_full()
        .gap(0.0)
        .child(rise(row().child(running), 0))
        .child(el().h(14.0))
        .child(rise(
            ui::heading("Point your backend at the relay", 26.0, k.c.text_base)
                .wrap()
                .key("onboarding-title"),
            1,
        ))
        .child(el().h(10.0))
        .child(rise(
            label(
                "Set this variable where your backend runs, then restart it. It keeps using its existing API key.",
                14.0,
                k.c.text_muted,
            )
            .wrap(),
            2,
        ))
        .child(el().h(22.0))
        .child(rise(command_card, 3))
        .child(el().h(10.0))
        .child(rise(
            label(
                format!("If your code sets baseUrl, baseURL or base_url, use {relay} there instead. Don't append /v1."),
                12.0,
                k.c.text_faint,
            )
            .wrap(),
            3,
        ))
        .child(el().h(20.0))
        .child(rise(listening, 4))
        .child(el().h(28.0))
        .child(rise(
            row()
                .w_full()
                .center_y()
                .child(back)
                .child(el().grow(1.0))
                .child(
                    primary("Open Inspector", false)
                        .on_press(finish)
                        .build(k)
                        .min_w(160.0)
                        .key("onboarding-finish"),
                ),
            5,
        ))
}

fn progress(step: u8, k: &Kit) -> El {
    let mut dots = row().gap(6.0).center_y();
    for i in PORT..=CONNECT {
        let active = i == step;
        dots = dots.child(
            el().h(6.0)
                .w(if active { 24.0 } else { 6.0 })
                .radius(3.0)
                .fill(if i <= step {
                    k.c.text_base
                } else {
                    k.c.layer_03
                })
                .motion(rune::Spring::new(0.35, 0.0)),
        );
    }
    dots.role("progressbar").label(format!("Step {step} of 3"))
}

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let k = ui::kit(s.dark);
    let ob = &s.onboarding;
    let body = match ob.step {
        WELCOME => welcome(&k),
        PORT => port_step(s, &k),
        CAPTURE => capture_step(s, &k),
        _ => connect_step(s, &k),
    };
    let header = row()
        .h(44.0)
        .w_full()
        .pl(cx.titlebar_inset() + 14.0)
        .pr(14.0)
        .center_y()
        .child(el().grow(1.0))
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
    let header = if cfg!(target_os = "macos") {
        header.window_drag()
    } else {
        header
    };
    let glow = if k.dark { 0.22 } else { 0.14 };
    col()
        .key("onboarding")
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .radial_gradient(
            [0.5, 0.0],
            [760.0, 520.0],
            [
                (0.0, pink().alpha(glow)),
                (0.55, violet().alpha(glow * 0.35)),
                (1.0, Color::TRANSPARENT),
            ],
        )
        .role("main")
        .label("Set up Jev Inspector")
        .child(header)
        .child(
            col().grow(1.0).min_h(0.0).w_full().center().px(32.0).child(
                col()
                    .w(if ob.step == WELCOME { 600.0 } else { 560.0 })
                    .child(body)
                    .key(format!("onboarding-step-{}", ob.step)),
            ),
        )
        .child(
            row()
                .h(56.0)
                .w_full()
                .center()
                .child(if ob.step == WELCOME || !ob.first_run {
                    el()
                } else {
                    progress(ob.step, &k)
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_state() -> State {
        let mut s = State::default();
        s.onboarding.app = true;
        s.onboarding.preview = true;
        s.offline_fixture = true;
        open(&mut s, WELCOME);
        s
    }

    #[test]
    fn first_run_walks_from_welcome_to_inspector() {
        let mut rt = rune::Runtime::new(
            app_state(),
            crate::view,
            (1100.0, 700.0),
            rune::Env {
                system_dark: true,
                scale: 1.0,
            },
        );
        rt.frame(0.0);
        let press = |rt: &mut rune::Runtime<State>, key: &str, t: f64| {
            let (x, y) = rt
                .find(key)
                .unwrap_or_else(|| panic!("{key} is not on screen"));
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(t);
        };
        press(&mut rt, "onboarding-start", 0.1);
        assert_eq!(rt.state.onboarding.step, PORT);
        rt.state.onboarding.port = TextEdit::new("80");
        refresh(&mut rt.state);
        assert!(matches!(rt.state.onboarding.status, PortStatus::Invalid(_)));
        rt.state.onboarding.port = TextEdit::new("9123");
        refresh(&mut rt.state);
        rt.frame(0.2);
        press(&mut rt, "onboarding-continue", 1.0);
        assert_eq!(rt.state.onboarding.step, CAPTURE);
        press(&mut rt, "onboarding-capture-metadata", 2.0);
        assert!(!rt.state.onboarding.capture);
        press(&mut rt, "onboarding-start-relay", 3.0);
        assert_eq!(rt.state.onboarding.step, CONNECT);
        assert_eq!(
            rt.state.onboarding.settings,
            Settings {
                port: 9123,
                capture_bodies: false
            }
        );
        assert_eq!(
            rt.state.workspace.prefs.get::<Settings>("relay"),
            Some(rt.state.onboarding.settings)
        );
        press(&mut rt, "onboarding-finish", 4.0);
        assert!(!rt.state.onboarding.open);
        assert!(rt.find("workspace").is_some());
    }

    #[test]
    fn toolbar_stops_and_restarts_the_hosted_relay() {
        let dir = std::env::temp_dir().join(format!("jev-stop-{}", jev_bridge::new_token()));
        let host = relay_host::Host::new(dir.join("connection.json"));
        let port =
            relay_host::suggest(24_000 + (std::process::id() % 2_000) as u16).expect("a free port");
        let settings = Settings {
            port,
            capture_bodies: true,
        };
        host.start(settings).unwrap();
        let mut s = State::default();
        s.onboarding.app = true;
        s.onboarding.settings = settings;
        s.offline_fixture = true;
        s.relay = Some(host.clone());
        let mut rt = rune::Runtime::new(
            s,
            crate::view,
            (1100.0, 700.0),
            rune::Env {
                system_dark: true,
                scale: 1.0,
            },
        );
        rt.frame(0.0);
        let press = |rt: &mut rune::Runtime<State>, key: &str, t: f64| {
            let (x, y) = rt
                .find(key)
                .unwrap_or_else(|| panic!("{key} is not on screen"));
            rt.pointer_down(x, y);
            rt.pointer_up(x, y);
            rt.frame(t);
        };
        press(&mut rt, "relay-toggle", 0.5);
        assert!(rt.state.onboarding.stopped);
        assert_eq!(host.running(), None);
        assert!(relay_host::check(port).is_ok(), "stopping frees the port");
        assert!(
            rt.find("relay-stopped").is_some(),
            "the stopped banner explains the state"
        );

        press(&mut rt, "relay-start", 1.0);
        assert!(!rt.state.onboarding.stopped);
        assert_eq!(host.running(), Some(settings));
        assert!(rt.find("relay-stopped").is_none());

        // A port taken while stopped sends the user to Relay settings.
        press(&mut rt, "relay-toggle", 1.5);
        let blocker = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        press(&mut rt, "relay-toggle", 2.0);
        assert!(rt.state.onboarding.stopped);
        assert!(rt.state.onboarding.open);
        assert_eq!(rt.state.onboarding.step, PORT);
        assert!(
            rt.state
                .onboarding
                .error
                .as_deref()
                .unwrap()
                .contains(&port.to_string())
        );
        drop(blocker);
        host.stop();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn route_command_targets_the_chosen_port() {
        assert_eq!(
            route_command(&relay_host::relay_url(9001)),
            if cfg!(windows) {
                "$env:TYPESAFE_BASE_URL = 'http://127.0.0.1:9001'"
            } else {
                "export TYPESAFE_BASE_URL=http://127.0.0.1:9001"
            }
        );
    }
}
