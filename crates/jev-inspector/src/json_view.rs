use crate::{State, label};
use jev_protocol::Capture;
use rune::{El, TextStyle, col, el, rich, row, text};
use rune_kit::{
    Button, ButtonVariant, Kit,
    syntax::{self, SyntaxTheme},
};

const PREVIEW_BYTES: usize = 16_000;

/// Keep copyable JSON separate from the bounded display and its explanatory text.
#[derive(Default)]
pub struct JsonDocument {
    source: Option<String>,
    end: usize,
    pub copied: bool,
    pub search_open: bool,
    pub search: rune::TextEdit,
    pub match_index: usize,
    pub tree: bool,
    pub(super) value: Option<serde_json::Value>,
    pub(super) outline: rune_kit::TreeModel<crate::json_tree::Property>,
}
impl JsonDocument {
    pub fn update(&mut self, capture: &Capture) {
        let source = capture
            .json
            .as_ref()
            .map(|v| serde_json::to_string_pretty(v).expect("JSON value serializes"));
        if self.source == source {
            return;
        }
        self.copied = false;
        self.match_index = 0;
        self.value = capture.json.clone();
        self.outline = capture
            .json
            .as_ref()
            .map(crate::json_tree::model)
            .unwrap_or_default();
        self.end = source.as_ref().map_or(0, |s| {
            let mut end = s.len().min(PREVIEW_BYTES);
            while !s.is_char_boundary(end) {
                end -= 1;
            }
            end
        });
        self.source = source;
    }
    fn preview(&self) -> &str {
        self.source.as_ref().map_or("", |s| &s[..self.end])
    }
    fn truncated(&self) -> bool {
        self.source.as_ref().is_some_and(|s| s.len() > self.end)
    }
    fn matches(&self) -> Vec<std::ops::Range<usize>> {
        if self.search.text.is_empty() {
            return vec![];
        }
        self.preview()
            .match_indices(&self.search.text)
            .take(1000)
            .map(|(i, s)| i..i + s.len())
            .collect()
    }
    pub fn find(&mut self, key: &str, delta: isize, tx: &mut rune::Tx) {
        let matches = self.matches();
        if matches.is_empty() {
            self.match_index = 0;
            return;
        }
        self.match_index =
            (self.match_index as isize + delta).rem_euclid(matches.len() as isize) as usize;
        let range = matches[self.match_index].clone();
        let line = self.preview()[..range.start]
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        tx.scroll_to_y(
            format!("{key}:scroll"),
            (line as f32 * 21.0 - 60.0).max(0.0),
        );
        let column = self.preview()[..range.start]
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .chars()
            .count();
        tx.scroll_to_x(
            format!("{key}:scroll"),
            (column as f32 * 7.8 - 180.0).max(0.0),
        );
    }
    pub fn view(
        &self,
        key: &str,
        get: fn(&mut State) -> &mut JsonDocument,
        k: &Kit,
        state: &State,
        cx: &rune::Cx<State>,
    ) -> El {
        let Some(source) = &self.source else {
            return col()
                .w_full()
                .padding(20.0)
                .gap(8.0)
                .radius(8.0)
                .fill(k.c.layer_01)
                .border(1.0, k.c.border_base)
                .child(label("No captured JSON", 14.0, k.c.text_base))
                .child(
                    label(
                        "This body has no JSON inspection copy. Check the capture notes above.",
                        12.0,
                        k.c.text_muted,
                    )
                    .wrap(),
                );
        };
        let preview = self.preview();
        let lines = preview.lines().count();
        let digits = lines.to_string().len();
        let numbers = (1..=lines)
            .map(|n| format!("{n:>digits$}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut font = TextStyle::new(13.0, 400.0).mono();
        font.line_height = 21.0;
        let matches = self.matches();
        let mut spans =
            SyntaxTheme::from_kit(k).spans(&syntax::highlight(Some("json"), preview), 0);
        for (i, range) in matches.iter().enumerate() {
            spans.push(rune::Span::new(range.start, range.end).bg(
                k.alpha(
                    k.c.warning.fg,
                    if i == self.match_index { 0.45 } else { 0.18 },
                ),
                0.0,
            ));
        }
        let header = row()
            .center_y()
            .gap(10.0)
            .px(14.0)
            .py(10.0)
            .fill(k.c.layer_02)
            .child(
                text("JSON")
                    .font(TextStyle::new(10.0, 600.0).mono())
                    .color(k.c.text_accent),
            )
            .child(
                Button::new(if self.tree { "Code" } else { "Tree" })
                    .variant(ButtonVariant::Ghost)
                    .on_press(move |s: &mut State| {
                        let doc = get(s);
                        doc.tree = !doc.tree;
                    })
                    .build(k)
                    .key(format!("{key}:tree"))
                    .help("Switch between formatted JSON and expandable properties"),
            )
            .child(
                crate::ui::action(&rune_kit::glyphs::SEARCH, "Find in JSON", self.search_open)
                    .disabled(self.tree)
                    .on_press(move |s: &mut State| {
                        let doc = get(s);
                        doc.search_open = !doc.search_open;
                        if !doc.search_open {
                            doc.search = rune::TextEdit::default();
                        } else {
                            doc.search.focused = true;
                        }
                    })
                    .build(k)
                    .key(format!("{key}:find"))
                    .help("Find text in this preview · Ctrl/Cmd+F"),
            )
            .child(
                label(
                    format!(
                        "{lines} lines{} · {}",
                        if self.truncated() { " shown" } else { "" },
                        crate::ui::bytes(source.len() as u64)
                    ),
                    11.0,
                    k.c.text_muted,
                )
                .grow(1.0),
            )
            .child(
                crate::ui::action(
                    if self.copied {
                        &rune_kit::glyphs::CHECK
                    } else {
                        &rune_kit::glyphs::COPY
                    },
                    if self.copied {
                        "JSON copied"
                    } else {
                        "Copy JSON"
                    },
                    self.copied,
                )
                .on_press_tx(move |s: &mut State, tx| {
                    let document = get(s);
                    if let Some(source) = &document.source {
                        tx.copy(source.clone());
                        document.copied = true;
                    }
                })
                .build(k)
                .key(format!("{key}:copy")),
            );
        let code = row()
            .key(format!("{key}:content"))
            .items_start()
            .child(
                text(numbers)
                    .font(font)
                    .color(k.c.text_faint)
                    .px(14.0)
                    .py(16.0)
                    .fill(k.c.layer_01)
                    .sticky_left(0.0)
                    .a11y_hidden(),
            )
            .child(
                col().px(16.0).py(16.0).child(
                    rich(preview, spans)
                        .key(format!("{key}:code"))
                        .font(font)
                        .color(k.c.text_base)
                        .selectable(true)
                        .selection_color(k.selection())
                        .role("code")
                        .label("Captured JSON"),
                ),
            );
        let mut panel = col()
            .key(key)
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .radius(9.0)
            .clip_rounded(9.0)
            .border(1.0, k.c.border_base)
            .fill(k.c.bg_deep)
            .child(header);
        if self.tree {
            return panel
                .child(el().w_full().h(1.0).fill(k.c.border_base))
                .child(crate::json_tree::view(state, key == "request-json", k, cx));
        }
        if self.search_open {
            let search_key = key.to_string();
            let enter_key = key.to_string();
            let mut controls = row()
                .px(12.0)
                .py(8.0)
                .gap(6.0)
                .center_y()
                .fill(k.c.layer_01)
                .child(
                    rune_kit::TextInput::new(&self.search)
                        .placeholder("Find in preview (case-sensitive)")
                        .label("Find in JSON")
                        .on_edit(move |s: &mut State| &mut get(s).search)
                        .on_change(move |s: &mut State, tx| {
                            let doc = get(s);
                            doc.match_index = 0;
                            doc.find(&search_key, 0, tx);
                        })
                        .on_key(move |s: &mut State, ev, tx| {
                            if ev.key == rune::Key::Enter {
                                get(s).find(&enter_key, if ev.mods.shift { -1 } else { 1 }, tx);
                                true
                            } else if ev.key == rune::Key::Escape {
                                let doc = get(s);
                                doc.search_open = false;
                                doc.search = rune::TextEdit::default();
                                tx.blur();
                                true
                            } else {
                                crate::workspace::on_key(s, ev, tx)
                            }
                        })
                        .build(k)
                        .grow(1.0)
                        .min_w(0.0)
                        .key(format!("{key}:search")),
                )
                .child(label(
                    if matches.is_empty() {
                        "0 matches".into()
                    } else {
                        format!(
                            "{} / {}{}",
                            self.match_index.min(matches.len() - 1) + 1,
                            matches.len(),
                            if matches.len() == 1000 { "+" } else { "" }
                        )
                    },
                    11.0,
                    k.c.text_muted,
                ));
            for (glyph, title, delta) in [
                (&crate::icons::UP, "Previous match", -1),
                (&crate::icons::DOWN, "Next match", 1),
            ] {
                let key = key.to_string();
                controls = controls.child(
                    crate::ui::action(glyph, title, false)
                        .disabled(matches.is_empty())
                        .on_press_tx(move |s: &mut State, tx| get(s).find(&key, delta, tx))
                        .build(k)
                        .help(if delta < 0 {
                            "Previous match · Shift+Enter"
                        } else {
                            "Next match · Enter"
                        }),
                );
            }
            controls = controls.child(
                crate::ui::action(&rune_kit::glyphs::CLOSE, "Close find", false)
                    .on_press(move |s: &mut State| {
                        let doc = get(s);
                        doc.search_open = false;
                        doc.search = rune::TextEdit::default();
                    })
                    .build(k)
                    .help("Close find"),
            );
            panel = panel.child(controls);
        }
        panel = panel
            .child(el().w_full().h(1.0).fill(k.c.border_base))
            .child(crate::request_scroll::area(
                cx,
                k,
                if key == "request-json" {
                    "request-json:scroll"
                } else {
                    "response-json:scroll"
                },
                if key == "request-json" {
                    "request-json:content"
                } else {
                    "response-json:content"
                },
                col()
                    .key(format!("{key}:scroll"))
                    .w_full()
                    .grow(1.0)
                    .min_h(0.0)
                    .scroll_xy()
                    .focusable(true)
                    .label("JSON code")
                    .child(code),
                true,
            ));
        if self.truncated() {
            panel=panel.child(label("Preview limited to 16,000 bytes. Copy JSON includes the complete captured JSON.",11.0,k.c.text_muted)
                .wrap().px(14.0).py(10.0).fill(k.c.layer_01));
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn truncated_unicode_preview_keeps_full_valid_copy_and_no_annotation_in_code() {
        let value = json!({"message":"🦀".repeat(5000),"ok":true,"missing":null});
        let capture = Capture {
            json: Some(value.clone()),
            ..Default::default()
        };
        let mut doc = JsonDocument::default();
        doc.update(&capture);
        assert!(doc.truncated());
        assert!(doc.preview().len() <= PREVIEW_BYTES);
        assert!(doc.source.as_ref().unwrap().starts_with(doc.preview()));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(doc.source.as_ref().unwrap()).unwrap(),
            value
        );
        assert!(!doc.preview().contains("Preview limited"));
        doc.copied = true;
        doc.update(&capture);
        assert!(doc.copied, "refresh must preserve copy feedback");
        doc.update(&Capture::default());
        assert!(doc.source.is_none());
        assert!(!doc.copied);
    }
}
