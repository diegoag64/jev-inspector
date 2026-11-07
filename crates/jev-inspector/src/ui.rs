use super::*;
use rune_kit::{ControlSize, IconButton, IconButtonVariant, PaletteFocus, PaletteSeed};

// Kit controls use Ink focus. Bare focusable regions must also set an explicit
// focus_ring: use text_faint for pane boundaries. Rune's fallback uses the system accent.
pub fn kit(dark: bool) -> Kit {
    let color = |d, l| rune::color::hex(if dark { d } else { l }).unwrap();
    let mut k = Kit::from_palette_focus(
        PaletteSeed {
            bg: color("#111111", "#f7f7f7"),
            fg: color("#eeeeee", "#202020"),
            accent: color("#eeeeee", "#202020"),
        },
        PaletteFocus::Ink,
    );
    k.c.bg_deep = color("#101010", "#ffffff");
    k.c.layer_01 = color("#191919", "#ffffff");
    k.c.layer_02 = color("#242424", "#f0f0f0");
    k.c.layer_03 = color("#303030", "#e7e7e7");
    k.c.border_base = color("#343434", "#dedede");
    k.c.border_muted = k.c.border_base;
    k.c.border_weak = k.c.border_base;
    k.c.text_faint = color("#a0a0a0", "#686868");
    k.c.bg_button_neutral = k.c.layer_02;
    k.c.bg_contrast = k.c.text_accent;
    k.c.text_contrast = color("#171717", "#ffffff");
    k.c.success.fg = color("#7fd6b1", "#19784e");
    k.c.warning.fg = color("#efc079", "#926017");
    k.c.danger.fg = color("#f194a0", "#b82d49");
    k
}
/// Shared 36-point action target. Rune supplies keyboard focus, tooltips and accessibility.
pub fn action(icon: &'static rune::Icon, label: &str, active: bool) -> IconButton {
    IconButton::new(icon)
        .label(label)
        .control_size(ControlSize::ExtraLarge)
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
        .control_size(ControlSize::Large)
        .variant(if active {
            ButtonVariant::Contrast
        } else {
            ButtonVariant::Ghost
        })
        .on_press(f)
        .build(k)
        .key(key)
}
pub fn badge(value: impl Into<String>, color: Color, k: &Kit) -> El {
    label(value, 11.0, color)
        .px(8.0)
        .py(4.0)
        .radius(5.0)
        .fill(k.alpha(color, 0.11))
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
