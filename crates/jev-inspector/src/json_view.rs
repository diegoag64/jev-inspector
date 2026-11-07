use crate::{State, label};
use jev_protocol::Capture;
use rune::{El, TextStyle, col, el, rich, row, text};
use rune_kit::{
    Button, ButtonVariant, Kit,
    syntax::{self, SyntaxTheme},
};

const PREVIEW_BYTES: usize = 16_000;

fn code_font() -> TextStyle {
    let mut font = TextStyle::new(13.0, 400.0).mono();
    font.line_height = 21.0;
    font
}

/// Keep copyable JSON separate from the bounded display and its explanatory text.
#[derive(Default)]
pub struct JsonDocument {
    source: Option<String>,
    start: usize,
    end: usize,
    matches: Vec<std::ops::Range<usize>>,
    reveal_version: u64,
    reveal_pending: bool,
    pub copied: bool,
    pub search_open: bool,
    pub search: rune::TextEdit,
    pub match_index: usize,
    pub tree: bool,
    pub(super) value: Option<serde_json::Value>,
    pub(super) outline: rune_kit::TreeModel<crate::json_tree::Property>,
}
impl JsonDocument {
    pub fn schedule_reveal(&mut self) {
        self.reveal_version = self.reveal_version.wrapping_add(1);
        self.reveal_pending = true;
    }
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
        self.source = source;
        self.set_window(0);
        self.refresh_matches();
        self.reveal_match();
        self.schedule_reveal();
    }
    fn set_window(&mut self, start: usize) {
        let source = self.source.as_deref().unwrap_or("");
        self.start = start.min(source.len());
        while !source.is_char_boundary(self.start) {
            self.start -= 1;
        }
        self.end = (self.start + PREVIEW_BYTES).min(source.len());
        while !source.is_char_boundary(self.end) {
            self.end -= 1;
        }
    }
    fn preview(&self) -> &str {
        self.source
            .as_ref()
            .map_or("", |s| &s[self.start..self.end])
    }
    fn truncated(&self) -> bool {
        self.source
            .as_ref()
            .is_some_and(|s| self.start > 0 || s.len() > self.end)
    }
    fn refresh_matches(&mut self) {
        self.matches = if self.search.text.is_empty() {
            vec![]
        } else {
            self.source
                .as_deref()
                .unwrap_or("")
                .match_indices(&self.search.text)
                .take(1001)
                .map(|(i, s)| i..i + s.len())
                .collect()
        };
        self.match_index = self
            .match_index
            .min(self.matches.len().min(1000).saturating_sub(1));
    }
    fn reveal_match(&mut self) {
        if let Some(range) = self.matches.get(self.match_index)
            && (range.start < self.start || range.end > self.end)
        {
            self.set_window(range.start.saturating_sub(2048));
        }
    }
    pub fn find(&mut self, key: &str, delta: isize, tx: &mut rune::Tx) {
        self.refresh_matches();
        if self.matches.is_empty() {
            self.match_index = 0;
            return;
        }
        self.match_index = (self.match_index as isize + delta)
            .rem_euclid(self.matches.len().min(1000) as isize) as usize;
        self.reveal_match();
        let range = self.matches[self.match_index].clone();
        let local_start = range.start - self.start;
        let line = self.preview()[..local_start]
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        tx.scroll_to_y(
            format!("{key}:scroll"),
            (line as f32 * 21.0 - 60.0).max(0.0),
        );
        let prefix = self.preview()[..local_start]
            .rsplit('\n')
            .next()
            .unwrap_or("");
        tx.scroll_to_x(
            format!("{key}:scroll"),
            (rune::text::advance(prefix, code_font()) - 180.0).max(0.0),
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
        let digits = (lines + source[..self.start].bytes().filter(|b| *b == b'\n').count())
            .to_string()
            .len();
        let first_line = source[..self.start].bytes().filter(|b| *b == b'\n').count() + 1;
        let numbers = (first_line..first_line + lines)
            .map(|n| format!("{n:>digits$}"))
            .collect::<Vec<_>>()
            .join("\n");
        let font = code_font();
        let matches = &self.matches;
        let mut spans =
            SyntaxTheme::from_kit(k).spans(&syntax::highlight(Some("json"), preview), 0);
        for (i, range) in matches.iter().enumerate() {
            if range.end <= self.start || range.start >= self.end {
                continue;
            }
            spans.push(
                rune::Span::new(
                    range.start.max(self.start) - self.start,
                    range.end.min(self.end) - self.start,
                )
                .bg(
                    k.alpha(
                        k.c.warning.fg,
                        if i == self.match_index { 0.45 } else { 0.18 },
                    ),
                    0.0,
                ),
            );
        }
        let header = row()
            .center_y()
            .gap(10.0)
            .px(14.0)
            .py(if state.workspace.window_height < 800.0 {
                6.0
            } else {
                10.0
            })
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
                        if !doc.tree {
                            doc.schedule_reveal();
                        }
                    })
                    .build(k)
                    .key(format!("{key}:tree"))
                    .help("Switch between formatted JSON and expandable properties"),
            )
            .child(
                crate::ui::action(&rune_kit::glyphs::SEARCH, "Find in JSON", self.search_open)
                    .on_press(move |s: &mut State| {
                        let doc = get(s);
                        let open = doc.tree || !doc.search_open;
                        doc.tree = false;
                        doc.search_open = open;
                        if !doc.search_open {
                            doc.search = rune::TextEdit::default();
                            doc.matches.clear();
                        } else {
                            doc.search.focused = true;
                            doc.schedule_reveal();
                        }
                    })
                    .build(k)
                    .key(format!("{key}:find"))
                    .help("Find text in the full capture · Ctrl/Cmd+F"),
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
                .py(if state.workspace.window_height < 800.0 {
                    5.0
                } else {
                    8.0
                })
                .gap(6.0)
                .center_y()
                .fill(k.c.layer_01)
                .child(
                    rune_kit::TextInput::new(&self.search)
                        .placeholder("Find in full capture (case-sensitive)")
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
                                doc.matches.clear();
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
                            matches.len().min(1000),
                            if matches.len() > 1000 { "+" } else { "" }
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
                        doc.matches.clear();
                    })
                    .build(k)
                    .help("Close find"),
            );
            panel = panel.child(controls);
        }
        let reveal_key = key.to_owned();
        let reveal = el()
            .key(format!("{key}:reveal:{}", self.reveal_version))
            .size(0.0, 0.0)
            .on_layout_tx(move |s: &mut State, _, tx| {
                let doc = get(s);
                if doc.reveal_pending {
                    doc.reveal_pending = false;
                    if doc.search_open && !doc.tree {
                        doc.find(&reveal_key, 0, tx);
                    }
                } else {
                    tx.live_only();
                }
            });
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
                    .child(reveal)
                    .child(code),
                true,
            ));
        if self.truncated() {
            let previous_key = key.to_owned();
            let next_key = key.to_owned();
            panel = panel.child(
                row()
                    .px(12.0)
                    .py(6.0)
                    .gap(6.0)
                    .center_y()
                    .fill(k.c.layer_01)
                    .child(
                        label(
                            format!(
                                "Bytes {}–{} of {} · Find and Copy use the full capture",
                                self.start + 1,
                                self.end,
                                source.len()
                            ),
                            11.0,
                            k.c.text_muted,
                        )
                        .wrap()
                        .grow(1.0),
                    )
                    .child(
                        crate::ui::action(&crate::icons::UP, "Previous JSON page", false)
                            .disabled(self.start == 0)
                            .on_press_tx(move |s: &mut State, tx| {
                                let doc = get(s);
                                doc.set_window(doc.start.saturating_sub(PREVIEW_BYTES));
                                tx.scroll_to_y(format!("{previous_key}:scroll"), 0.0);
                            })
                            .build(k)
                            .key(format!("{key}:previous-page")),
                    )
                    .child(
                        crate::ui::action(&crate::icons::DOWN, "Next JSON page", false)
                            .disabled(self.end == source.len())
                            .on_press_tx(move |s: &mut State, tx| {
                                let doc = get(s);
                                doc.set_window(doc.end);
                                tx.scroll_to_y(format!("{next_key}:scroll"), 0.0);
                            })
                            .build(k)
                            .key(format!("{key}:next-page")),
                    ),
            );
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn full_capture_search_reveals_tail_and_pages_at_unicode_boundaries() {
        let mut doc = JsonDocument::default();
        doc.update(&Capture {
            json: Some(json!({"a": "🦀".repeat(6000), "z": "unique tail marker"})),
            ..Default::default()
        });
        assert!(!doc.preview().contains("unique tail marker"));
        doc.search = rune::TextEdit::new("unique tail marker");
        doc.find("response-json", 0, &mut rune::Tx::default());
        assert_eq!(doc.matches.len(), 1);
        assert!(doc.start > 0 && doc.preview().contains("unique tail marker"));
        assert!(doc.preview().len() <= PREVIEW_BYTES);
        doc.search = rune::TextEdit::new("not in this capture");
        doc.find("response-json", 0, &mut rune::Tx::default());
        assert!(doc.matches.is_empty());
        doc.set_window(0);
        let mut reconstructed = String::new();
        loop {
            reconstructed.push_str(doc.preview());
            if doc.end == doc.source.as_ref().unwrap().len() {
                break;
            }
            doc.set_window(doc.end);
        }
        assert_eq!(Some(&reconstructed), doc.source.as_ref());
    }
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
