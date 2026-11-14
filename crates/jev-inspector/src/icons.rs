//! Monochrome vector actions, rendered by Rune at the current display scale.
use rune::Icon;

macro_rules! glyph {
    ($name:ident, $body:literal) => {
        pub static $name: Icon = Icon { name: concat!("jev-", stringify!($name)), view: 24.0,
            body: concat!(r#"<g fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">"#, $body, "</g>") };
    };
}
glyph!(
    PAUSE,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M8 5v14M16 5v14"/>"#
);
glyph!(
    PLAY,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="m8 4 12 8-12 8Z"/>"#
);
glyph!(
    FOLLOW,
    r#"<circle fill="none" stroke="currentColor" stroke-width="1.7" cx="12" cy="12" r="6"/><circle fill="none" stroke="currentColor" stroke-width="1.7" cx="12" cy="12" r="2"/><path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" d="M12 2v4m0 12v4M2 12h4m12 0h4"/>"#
);
glyph!(
    CLEAR,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="m4 15 9-11 8 7-9 10H9l-5-4v-2Zm3-4 8 7M12 21h9"/>"#
);
glyph!(
    OPEN,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M3 8V5h6l2 3h10l-3 12H3V8Zm0 12 4-9h14"/>"#
);
glyph!(
    EXPORT,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M12 16V3m-5 5 5-5 5 5M4 15v6h16v-6"/>"#
);
glyph!(
    HELP,
    r#"<circle fill="none" stroke="currentColor" stroke-width="1.7" cx="12" cy="12" r="9"/><path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M9.5 9a2.5 2.5 0 0 1 5 0c0 2-2.5 2-2.5 4m0 3v.1"/>"#
);
glyph!(
    SUN,
    r#"<circle fill="none" stroke="currentColor" stroke-width="1.7" cx="12" cy="12" r="4"/><path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>"#
);
glyph!(
    MOON,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M20 15.5A9 9 0 0 1 8.5 4 9 9 0 1 0 20 15.5Z"/>"#
);
glyph!(
    EXPAND,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5"/>"#
);
glyph!(
    RESTORE,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M3 8h5V3m8 0v5h5M8 21v-5H3m13 5v-5h5"/>"#
);
glyph!(
    UP,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="m6 14 6-6 6 6"/>"#
);
glyph!(
    DOWN,
    r#"<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="m6 10 6 6 6-6"/>"#
);
glyph!(PULSE, r#"<path d="M3 12h4l3-7 4 14 3-7h4"/>"#);
glyph!(
    NODES,
    r#"<circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="12" cy="18" r="2.5"/><path d="M7.4 8.2 10.6 15.8M16.6 8.2 13.4 15.8M8.5 6h7"/>"#
);
glyph!(
    SHIELD,
    r#"<path d="M12 3 19 6v6c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6Z"/><path d="m9 12 2 2 4-4"/>"#
);
glyph!(
    SERVER,
    r#"<rect x="4" y="4" width="16" height="7" rx="2"/><rect x="4" y="13" width="16" height="7" rx="2"/><path d="M8 7.5h.01M8 16.5h.01"/>"#
);
glyph!(
    CLOUD,
    r#"<path d="M7 18h10a4 4 0 0 0 .6-7.96A6 6 0 0 0 6.1 11.6 3.25 3.25 0 0 0 7 18Z"/>"#
);
glyph!(
    RELAY,
    r#"<circle cx="12" cy="12" r="4"/><path d="M3 12h5M16 12h5"/><circle cx="12" cy="12" r="1" fill="currentColor"/>"#
);
glyph!(
    STOP,
    r#"<rect x="6" y="6" width="12" height="12" rx="2.5"/>"#
);
