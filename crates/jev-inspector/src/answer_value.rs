use super::*;
use serde_json::Value;

pub fn view(kind: &str, value: &Value, k: &Kit) -> El {
    if kind == "noul"
        && let Some(p) = value.as_f64().filter(|p| (0.0..=1.0).contains(p))
    {
        return col()
            .gap(10.0)
            .child(
                row()
                    .center_y()
                    .gap(12.0)
                    .child(label(format!("{:.1}%", p * 100.0), 24.0, k.c.text_base))
                    .child(label("Probability of true", 12.0, k.c.text_muted)),
            )
            .child(
                row()
                    .h(6.0)
                    .radius(3.0)
                    .clip_rounded(3.0)
                    .child(
                        el().h_full()
                            .grow((p as f32).max(0.0001))
                            .fill(k.c.text_base),
                    )
                    .child(
                        el().h_full()
                            .grow((1.0 - p as f32).max(0.0001))
                            .fill(k.c.layer_03),
                    ),
            )
            .child(
                row()
                    .child(label("False: 0%", 10.0, k.c.text_faint).grow(1.0))
                    .child(label("True: 100%", 10.0, k.c.text_faint)),
            );
    }
    if kind == "box" {
        let coordinates: Option<Vec<f64>> = ["left", "top", "right", "bottom"]
            .iter()
            .map(|name| value[*name].as_f64())
            .collect();
        if let Some(c) = coordinates
            && c.iter().all(|v| (0.0..=1.0).contains(v))
            && c[2] >= c[0]
            && c[3] >= c[1]
        {
            let mut facts = col().grow(1.0).gap(8.0);
            for (name, v) in ["Left", "Top", "Right", "Bottom"].iter().zip(&c) {
                facts = facts.child(
                    row()
                        .gap(10.0)
                        .child(label(*name, 11.0, k.c.text_muted).grow(1.0))
                        .child(ui::mono(format!("{v:.3}"), 12.0, k.c.text_base)),
                );
            }
            return col()
                .gap(10.0)
                .child(
                    row()
                        .gap(18.0)
                        .center_y()
                        .child(
                            el().size(200.0, 100.0)
                                .radius(5.0)
                                .fill(k.c.layer_02)
                                .border(1.0, k.c.border_base)
                                .a11y_hidden()
                                .child(
                                    el().absolute(c[0] as f32 * 200.0, c[1] as f32 * 100.0)
                                        .size(
                                            (c[2] - c[0]) as f32 * 200.0,
                                            (c[3] - c[1]) as f32 * 100.0,
                                        )
                                        .radius(2.0)
                                        .fill(k.alpha(k.c.text_base, 0.14))
                                        .border(1.0, k.c.text_base),
                                ),
                        )
                        .child(facts),
                )
                .child(
                    label(
                        "Normalized coordinates · image not retained",
                        11.0,
                        k.c.text_faint,
                    )
                    .wrap(),
                );
        }
    }
    let text = value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string());
    label(
        clip(&text, 500),
        if kind == "score" { 28.0 } else { 20.0 },
        k.c.text_base,
    )
    .wrap()
    .selectable(true)
}
