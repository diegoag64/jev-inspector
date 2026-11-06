use super::*;
use rune_kit::{Tree, TreeItem, TreeModel, TreeStyle};
use serde_json::Value;

pub struct Property {
    label: String,
    kind: &'static str,
}
pub fn model(value: &Value) -> TreeModel<Property> {
    fn add(
        model: &mut TreeModel<Property>,
        value: &Value,
        name: &str,
        path: String,
        parent: Option<&str>,
        depth: usize,
    ) {
        if model.len() >= 4096 || depth > 64 {
            return;
        }
        let (kind, summary) = match value {
            Value::Object(v) => ("object", format!("{{ {} fields }}", v.len())),
            Value::Array(v) => ("array", format!("[ {} items ]", v.len())),
            Value::String(v) => ("string", serde_json::to_string(&clip(v, 180)).unwrap()),
            Value::Bool(v) => ("boolean", v.to_string()),
            Value::Null => ("null", "null".into()),
            Value::Number(v) => ("number", v.to_string()),
        };
        model.insert(
            parent,
            &path,
            Property {
                label: format!("{}: {summary}", clip(name, 100)),
                kind,
            },
        );
        match value {
            Value::Object(v) => {
                for (name, value) in v.iter().take(4096) {
                    let child = format!("{path}/{}", name.replace('~', "~0").replace('/', "~1"));
                    add(model, value, name, child, Some(&path), depth + 1);
                    if model.len() >= 4096 {
                        break;
                    }
                }
            }
            Value::Array(v) => {
                for (i, value) in v.iter().take(4096).enumerate() {
                    add(
                        model,
                        value,
                        &i.to_string(),
                        format!("{path}/{i}"),
                        Some(&path),
                        depth + 1,
                    );
                    if model.len() >= 4096 {
                        break;
                    }
                }
            }
            _ => {}
        }
        if depth <= 1 {
            model.toggle(&path);
        }
    }
    let mut model = TreeModel::new().key("json-properties");
    add(&mut model, value, "$", "$".into(), None, 0);
    model
}
pub fn view(s: &State, request: bool, k: &Kit, cx: &Cx<State>) -> El {
    type Read = fn(&State) -> &TreeModel<Property>;
    let read: Read = if request {
        |s| &s.request_text.outline
    } else {
        |s| &s.response_text.outline
    };
    let edit: fn(&mut State) -> &mut TreeModel<Property> = if request {
        |s| &mut s.request_text.outline
    } else {
        |s| &mut s.response_text.outline
    };
    let doc = if request {
        &s.request_text
    } else {
        &s.response_text
    };
    let selected = doc.outline.selected().next();
    let colors = k.c;
    let mut style = TreeStyle::from_kit(k);
    style.row_height = 30.0;
    style.indent = 18.0;
    col()
        .grow(1.0)
        .min_h(0.0)
        .gap(8.0)
        .child(
            row()
                .px(10.0)
                .py(6.0)
                .gap(4.0)
                .child(ui::button(
                    "Expand all",
                    "json-expand-all",
                    false,
                    k,
                    move |s| edit(s).expand_all(),
                ))
                .child(ui::button(
                    "Collapse all",
                    "json-collapse-all",
                    false,
                    k,
                    move |s| edit(s).collapse_all(),
                ))
                .child(el().grow(1.0))
                .child(
                    Button::new("Copy value")
                        .variant(ButtonVariant::Ghost)
                        .disabled(selected.is_none())
                        .on_press_tx(move |s: &mut State, tx| {
                            let doc = if request {
                                &s.request_text
                            } else {
                                &s.response_text
                            };
                            if let Some(id) = doc.outline.selected().next()
                                && let Some(value) = doc
                                    .value
                                    .as_ref()
                                    .and_then(|v| v.pointer(id.strip_prefix('$').unwrap_or("")))
                            {
                                tx.copy(serde_json::to_string_pretty(value).unwrap());
                                s.workspace.notice = Some((
                                    format!("Copied JSON value at {}", clip(id, 100)),
                                    false,
                                ));
                            }
                        })
                        .build(k)
                        .key("copy-json-value"),
                ),
        )
        .child(
            col()
                .grow(1.0)
                .min_h(0.0)
                .pr(16.0)
                .child(
                    Tree::new(read, edit, |n: &Property| &n.label)
                        .row(move |n, _| {
                            TreeItem::new(&n.label).label_color(match n.kind {
                                "string" => colors.success.fg,
                                "number" | "boolean" => colors.warning.fg,
                                "null" => colors.text_faint,
                                _ => colors.text_base,
                            })
                        })
                        .style(style)
                        .build(s, k)
                        .label(if request {
                            "Request JSON properties"
                        } else {
                            "Response JSON properties"
                        })
                        .grow(1.0)
                        .min_h(0.0),
                )
                .child(request_scroll::vertical(
                    cx,
                    k,
                    0.0,
                    "json-properties/viewport",
                    "json-properties/viewport/rows",
                    "json-tree-scrollbar",
                    "json-tree-scroll-thumb",
                )),
        )
        .child(
            label(
                format!(
                    "{} properties · Bounded preview · Copy keeps full values",
                    doc.outline.len()
                ),
                10.0,
                k.c.text_faint,
            )
            .wrap()
            .px(14.0)
            .py(8.0),
        )
}
#[cfg(test)]
mod tests {
    #[test]
    fn bounds_large_arrays_and_preserves_json_pointer_paths() {
        let value = serde_json::json!({"a/b": {"~": "value"}});
        let mut model = super::model(&value);
        model.select("$/a~1b/~0");
        assert_eq!(model.selected().next(), Some("$/a~1b/~0"));
        assert_eq!(value.pointer("/a~1b/~0").unwrap(), "value");
        let large = serde_json::Value::Array(vec![serde_json::Value::Null; 5000]);
        assert_eq!(super::model(&large).len(), 4096);
    }
}
