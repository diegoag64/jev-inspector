use super::*;
use rune_kit::{ControlSize, IconButton, IconButtonVariant, PaletteFocus, PaletteSeed};

/// Sans and mono slots. SF Pro (with its optical sizes and `trak` tracking) and SF Mono
/// on macOS; Inter and the platform's monospace elsewhere.
pub const FAMILIES: (&str, &str) = ("System Font, Inter", "ui-monospace");

// Corner radii, drawn with Apple's continuous curve. Nested surfaces step down so
// inner corners stay concentric with their container.
/// Workspace panes.
pub const PANEL_RADIUS: f32 = 12.0;
/// Cards and code surfaces inside a pane.
pub const CARD_RADIUS: f32 = 10.0;
/// Tiles, fields and segmented tracks.
pub const INSET_RADIUS: f32 = 8.0;

// Kit controls use Ink focus. Bare focusable regions must also set an explicit
// focus_ring: use text_faint for pane boundaries. Rune's fallback uses the system accent.
//
// The palette follows Apple's grouped system surfaces (window, pane, card, fill)
// with neutral ink as the accent. Status hues are Apple's increased-contrast
// variants, chosen to keep 4.5:1 on panes, cards and selected Traffic rows.
pub fn kit(dark: bool) -> Kit {
    let color = |d, l| rune::color::hex(if dark { d } else { l }).unwrap();
    let mut k = Kit::from_palette_focus(
        PaletteSeed {
            bg: color("#000000", "#f2f2f7"),
            fg: color("#f5f5f7", "#1d1d1f"),
            accent: color("#f5f5f7", "#1d1d1f"),
        },
        PaletteFocus::Ink,
    );
    k.c.bg_deep = color("#242426", "#f5f5f7");
    k.c.layer_01 = color("#1c1c1e", "#ffffff");
    k.c.layer_02 = color("#2c2c2e", "#eeeef0");
    k.c.layer_03 = color("#3a3a3c", "#e3e3e8");
    k.c.border_base = color("#38383a", "#e3e3e8");
    k.c.border_muted = k.c.border_base;
    k.c.border_weak = k.c.border_base;
    k.c.text_muted = color("#aeaeb2", "#5e5e63");
    k.c.text_faint = color("#8e8e93", "#6e6e73");
    k.c.bg_button_neutral = k.c.layer_02;
    k.c.bg_contrast = k.c.text_accent;
    k.c.text_contrast = color("#1c1c1e", "#ffffff");
    k.c.success.fg = color("#30d158", "#1a7033");
    k.c.warning.fg = color("#ffb340", "#a84a00");
    k.c.danger.fg = color("#ff6961", "#c8102e");
    k.radius.xs = 4.0;
    k.radius.sm = 6.0;
    k.radius.md = 7.0;
    k.radius.lg = 8.0;
    k.radius.xl = 10.0;
    k.elevation.button_neutral.clear();
    k.elevation.button_contrast.clear();
    k
}

/// Text fields paint their own `bg_base`; give them the grouped fill tone so they
/// sit on panes like native search fields instead of punching through to the window.
pub fn field_kit(k: &Kit) -> Kit {
    let mut field = k.clone();
    field.c.bg_base = k.c.layer_02;
    field
}

/// Regular-weight text. SF's optical sizes carry the hierarchy at small sizes.
pub fn text_style(size: f32) -> rune::TextStyle {
    rune::TextStyle::new(size, 400.0)
}

/// Semibold titles: pane names, section and card headings.
pub fn heading(value: impl Into<String>, size: f32, color: Color) -> El {
    text(value)
        .font(rune::TextStyle::new(size, 600.0))
        .color(color)
}

/// Figures in the system face with tabular digits, so values stay aligned
/// without switching to a code font.
pub fn figure(value: impl Into<String>, size: f32, weight: f32, color: Color) -> El {
    text(value)
        .font(rune::TextStyle::new(size, weight).features("'tnum' 1"))
        .color(color)
}

/// A pane, card or tile surface with continuous corners.
pub fn surface(e: El, radius: f32, fill: Color) -> El {
    e.radius(radius).continuous_corners().fill(fill)
}

/// The track of a segmented control. Children come from [`segment`].
pub fn segmented(k: &Kit) -> El {
    surface(row(), INSET_RADIUS, k.c.layer_02)
        .padding(2.0)
        .gap(2.0)
        .center_y()
}

/// The raised selection inside a segmented track.
pub fn segment_fill(k: &Kit) -> Color {
    rune::color::hex(if k.dark { "#5a5a5e" } else { "#ffffff" }).unwrap()
}

/// Lift the selected segment off its track, like a native segmented control.
pub fn segment_lift(e: El, active: bool, k: &Kit) -> El {
    if active {
        e.radius(INSET_RADIUS - 2.0)
            .continuous_corners()
            .fill(segment_fill(k))
            .shadow(
                1.0,
                3.0,
                k.alpha(k.c.black, if k.dark { 0.3 } else { 0.12 }),
            )
    } else {
        e.radius(INSET_RADIUS - 2.0)
            .continuous_corners()
            .fill(Color::TRANSPARENT)
    }
}

/// One option in a [`segmented`] track.
pub fn segment(
    name: &str,
    key: &str,
    active: bool,
    k: &Kit,
    f: impl Fn(&mut State) + 'static,
) -> El {
    segment_lift(
        Button::new(name)
            .control_size(ControlSize::Regular)
            .variant(ButtonVariant::Ghost)
            .on_press(f)
            .build(k),
        active,
        k,
    )
    .key(key)
}
/// Compact desktop action target; Rune supplies focus, tooltips and accessibility.
pub fn action(icon: &'static rune::Icon, label: &str, active: bool) -> IconButton {
    IconButton::new(icon)
        .label(label)
        .control_size(ControlSize::Large)
        .variant(if active {
            IconButtonVariant::Contrast
        } else {
            IconButtonVariant::Ghost
        })
}
pub fn button(
    name: &str,
    key: &str,
    active: bool,
    k: &Kit,
    f: impl Fn(&mut State) + 'static,
) -> El {
    Button::new(name)
        .control_size(ControlSize::Regular)
        .variant(if active {
            ButtonVariant::Neutral
        } else {
            ButtonVariant::Ghost
        })
        .on_press(f)
        .build(k)
        .fill(if active {
            k.c.layer_03
        } else {
            Color::TRANSPARENT
        })
        .continuous_corners()
        .key(key)
}
/// A capsule status tag in sentence case.
pub fn badge(value: impl Into<String>, color: Color, k: &Kit) -> El {
    text(value)
        .font(rune::TextStyle::new(11.0, 590.0))
        .color(color)
        .px(8.0)
        .py(3.0)
        .radius(10.0)
        .fill(k.alpha(color, if k.dark { 0.16 } else { 0.1 }))
}
pub fn rule(k: &Kit) -> El {
    el().w_full().h(1.0).fill(k.c.border_base)
}
pub fn mono(value: impl Into<String>, size: f32, color: Color) -> El {
    text(value)
        .font(rune::TextStyle::new(size, 400.0).mono())
        .color(color)
}
pub fn bytes(value: u64) -> String {
    if value >= 1024 * 1024 {
        format!("{:.1} MB", value as f64 / 1048576.0)
    } else if value >= 1024 {
        format!("{:.1} KB", value as f64 / 1024.0)
    } else {
        format!("{value} B")
    }
}
pub fn duration(value: Option<f64>) -> String {
    value
        .map(|v| {
            if v >= 1000.0 {
                format!("{:.2} s", v / 1000.0)
            } else {
                format!("{v:.0} ms")
            }
        })
        .unwrap_or_else(|| "In flight".into())
}
pub fn time(ms: u64) -> String {
    let s = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        s / 3600 % 24,
        s / 60 % 60,
        s % 60,
        ms % 1000
    )
}
pub fn timestamp(ms: u64) -> String {
    // Civil date from days since the Unix epoch, in UTC (Gregorian calendar).
    let z = (ms / 86_400_000) as i64 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {}", time(ms))
}
pub fn status_color(s: &Summary, k: &Kit) -> Color {
    if failed(s) {
        k.c.danger.fg
    } else if s.outcome == Outcome::Pending {
        k.c.warning.fg
    } else {
        k.c.success.fg
    }
}
