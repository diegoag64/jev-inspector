use rune::{Color, El, TextStyle, text};
use rune_kit::Kit;

/// Jev's original dark diagram palette, plus a light adaptation with the same accent.
pub fn jev_theme(base: &Kit) -> Kit {
    let mut k = base.clone();
    let hex =
        |dark, light| rune::color::hex(if base.dark { dark } else { light }).expect("static color");
    k.c.bg_base = hex("#0d0a0f", "#fffafd");
    k.c.layer_01 = hex("#0d0a0f", "#f7f1f7");
    k.c.layer_02 = hex("#463449", "#dfcddd");
    k.c.text_base = hex("#f5f3f6", "#291d2b");
    k.c.text_muted = hex("#b7b1bc", "#716274");
    k.c.text_faint = hex("#968592", "#8c758a");
    k.c.text_accent = hex("#e551ba", "#b5278e");
    k.c.border_focus = k.c.text_accent;
    k.c.border_base = hex("#302634", "#ded0df");
    k.c.border_muted = hex("#645a68", "#b7a3b8");
    k.c.border_strong = hex("#f5f3f6", "#705b73");
    k
}

pub(crate) fn label(value: impl Into<String>, size: f32, color: Color) -> El {
    text(value).font(TextStyle::new(size, 400.0)).color(color)
}
pub(crate) fn alpha(mut c: Color, a: f32) -> Color {
    c.a *= a;
    c
}

/// Compact percent formatting; the underlying probability remains unchanged.
pub fn percent(value: f64) -> String {
    let value = value * 100.0;
    let magnitude = if value == 0.0 {
        0.0
    } else {
        value.abs().log10().floor()
    };
    let decimals = (5.0 - magnitude).clamp(0.0, 12.0) as usize;
    let s = format!("{value:.decimals$}");
    format!(
        "{}%",
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.')
        } else {
            &s
        }
    )
}
