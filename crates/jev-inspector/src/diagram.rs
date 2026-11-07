use crate::{State, label};
use jev_protocol::Exchange;
use rune::{Cx, El, col, row};
use rune_diagram::{Diagram, Edge, Graph, Node, NodeBody, NodeStatus, ResultContent, percent};
use rune_kit::{Button, ButtonVariant, Kit};
use serde_json::Value;

/// Rune's Tab traversal runs after element key handlers. Restore the active
/// card before traversal can enter it from either direction after a manual pan.
pub fn reveal_keyboard_card(s: &mut State) {
    let Some(detail) = s.detail.as_ref() else {
        return;
    };
    let questions = detail
        .request
        .json
        .as_ref()
        .and_then(|v| v.get("questions"))
        .and_then(Value::as_object);
    let selected = questions.and_then(|q| {
        q.get_key_value(&s.diagram_question)
            .or_else(|| q.iter().next())
    });
    let extra = s.diagram.content.visible("categories").saturating_sub(12);
    let (graph, _) = graph(detail, selected, extra);
    let id = s.diagram.selected.as_deref().unwrap_or("answer");
    if let Some(node) = graph.nodes.iter().find(|n| n.id == id) {
        s.diagram.reveal(node.rect);
    }
}

pub fn view(s: &State, detail: &Exchange, k: &Kit, cx: &Cx<State>, layers: &mut Vec<El>) -> El {
    let questions = detail
        .request
        .json
        .as_ref()
        .and_then(|v| v.get("questions"))
        .and_then(Value::as_object);
    let selected = questions.and_then(|q| {
        q.get_key_value(&s.diagram_question)
            .or_else(|| q.iter().next())
    });
    let names: Vec<&str> = questions
        .map(|q| q.keys().map(String::as_str).collect())
        .unwrap_or_default();
    let mut question_layers = Vec::new();
    // Measure with the same font as Rune's options, including room for padding,
    // the selection check and the list's scrollbar. Keep the trigger stable
    // across selections by sizing from all options rather than the current one.
    let label_width = names
        .iter()
        .map(|name| rune::text::measure(name, &[], k.ty.ui_compact, None).0)
        .fold(0.0_f32, f32::max);
    let desired_width = (label_width + 54.0).clamp(140.0, 480.0);
    let menu_width = cx
        .window_layout("diagram-question-trigger")
        .map_or(desired_width, |r| {
            desired_width.min((s.workspace.window_width - r[0] - 24.0).max(1.0))
        });
    let selector = rune_kit::Select::new("diagram-question", &names)
        .label(format!("Question ({} total)", names.len()))
        .current(selected.map(|(name, _)| name.as_str()))
        .placeholder("No questions")
        .width(desired_width.min(260.0))
        .placement(rune_kit::Placement::BottomStart)
        .appearance(rune_kit::SelectAppearance::Field)
        .disabled(names.is_empty())
        .open(s.diagram_questions_open)
        .on_toggle(|s: &mut State| s.diagram_questions_open = !s.diagram_questions_open)
        .on_close(|s: &mut State| s.diagram_questions_open = false)
        .on_choose(|s: &mut State, name, _| {
            s.diagram_question = name.to_owned();
            s.diagram_questions_open = false;
            s.diagram.reset();
        })
        .build(k, &mut question_layers);
    // Preserve Rune's selection, scrolling and keyboard behavior; only replace
    // its fixed menu bounds with the content width calculated above.
    if s.diagram_questions_open
        && let Some(menu) = question_layers.pop()
    {
        question_layers.push(menu.w(menu_width).min_w(menu_width).max_w(menu_width));
    }
    layers.extend(question_layers);
    let toolbar = row()
        .gap(6.0)
        .center_y()
        .child(label("Question", 11.0, k.c.text_muted))
        .child(selector.min_w(0.0))
        .child(rune::el().grow(1.0))
        .child(crate::ui::action(&crate::icons::HELP, "Diagram controls", false)
            .on_press(|s: &mut State| {s.workspace.setup_help=false; s.workspace.help=true;})
            .build(k).key("diagram-help")
            .help("Drag to pan. Wheel to zoom. Tab enters cards; Up/Down browse them. Open Help for keyboard controls."))
        .child(
            Button::new(if s.workspace.detail_expanded {
                "Restore"
            } else {
                "Expand"
            })
            .icon(if s.workspace.detail_expanded {
                &crate::icons::RESTORE
            } else {
                &crate::icons::EXPAND
            })
            .variant(ButtonVariant::Ghost)
            .on_press(State::toggle_expanded)
            .build(k)
            .key("diagram-expand"),
        );
    let extra = s.diagram.content.visible("categories").saturating_sub(12);
    let (graph, remaining) = graph(detail, selected, extra);
    let mut root = col().w_full().grow(1.0).min_h(0.0).gap(10.0).child(toolbar);
    if remaining > 0 && extra < 117 {
        let (_, next_remaining) = self::graph(detail, selected, extra + 3);
        let additions = remaining.saturating_sub(next_remaining);
        root = root.child(
            Button::new(format!(
                "Show {} more {} ({remaining} remaining)",
                additions,
                if additions == 1 {
                    "category"
                } else {
                    "categories"
                }
            ))
            .on_press(|s: &mut State| s.diagram.content.more_by("categories".into(), 3))
            .build(k)
            .key("diagram-more-categories"),
        );
    } else if remaining > 0 {
        root=root.child(label(format!("{remaining} categories outside the diagram preview limit; inspect the Response tab."),11.0,k.c.text_muted));
    }
    let notes: Vec<&str> = detail
        .request
        .notes
        .iter()
        .chain(&detail.response.notes)
        .map(String::as_str)
        .collect();
    if !notes.is_empty() {
        let message = notes.join(" · ");
        root = root.child(
            label(
                format!("Capture notes ({}) · {message}", notes.len()),
                11.0,
                k.c.text_muted,
            )
            .truncate()
            .help(message),
        );
    }
    root.child(
        Diagram::new("request-diagram", &graph, &s.diagram, |s: &mut State| {
            &mut s.diagram
        })
        .entry_card("answer")
        .compact(s.workspace.window_height < 800.0)
        .build(k),
    )
    .child(if s.workspace.window_height < 800.0 {
        rune::el()
    } else {
        label(
            "Captured data and returned values · your application decides what runs next",
            11.0,
            k.c.text_muted,
        )
    })
}

struct Output {
    title: String,
    eyebrow: String,
    detail: String,
    active: bool,
}

pub(super) fn graph(
    detail: &Exchange,
    selected: Option<(&String, &Value)>,
    extra: usize,
) -> (Graph, usize) {
    let request = detail.request.json.as_ref();
    let response = detail.response.json.as_ref();
    let answer = selected.and_then(|(key, _)| response?.get("answers")?.get(key));
    let state = request.and_then(|v| v.get("state"));
    let (name, question) = selected.map_or(("Question not captured", None), |(key, v)| {
        (key.as_str(), Some(v))
    });
    let kind = question
        .and_then(|v| v.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut values: Vec<(String, f64)> = answer
        .and_then(|v| v.get("probabilities"))
        .and_then(Value::as_object)
        .map(|p| {
            p.iter()
                .filter_map(|(key, p)| p.as_f64().map(|p| (key.clone(), p)))
                .collect()
        })
        .unwrap_or_default();
    values.sort_by(|a, b| b.1.total_cmp(&a.1));
    let choice = answer
        .and_then(|v| v.get("choice"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let compact = kind == "choice" && values.len() > 6;
    let mut shown = if compact {
        values
            .iter()
            .take((3 + extra).min(120))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        values.iter().take(120).cloned().collect()
    };
    // A returned category is always shown, even when it is outside the highest probabilities.
    if compact
        && let Some(chosen) = &choice
        && !shown.iter().any(|(key, _)| key == chosen)
        && let Some(item) = values.iter().find(|(key, _)| key == chosen)
    {
        shown.push(item.clone());
    }
    // At most 120 leading categories plus the explicit returned choice; raw JSON stays available.
    let remaining = values.len().saturating_sub(shown.len());
    let mut outputs = vec![];
    let status = if detail.summary.status.is_some_and(|s| s >= 400) {
        NodeStatus::Error
    } else if answer.is_some() {
        NodeStatus::Success
    } else {
        NodeStatus::Unavailable
    };
    let body = if let Some(answer) = answer {
        let mut result = ResultContent {
            confidence: answer.get("confidence").and_then(Value::as_f64),
            ..Default::default()
        };
        match kind {
            "choice" => {
                result.probabilities = shown.clone();
                result.selected = choice.clone();
                if compact {
                    result.caption = Some(format!(
                        "Showing {} of {} probabilities",
                        shown.len(),
                        values.len()
                    ));
                }
                for (key, p) in &shown {
                    let active = choice.as_ref() == Some(key);
                    outputs.push(Output {
                        title: key.clone(),
                        eyebrow: if active {
                            "RETURNED CHOICE"
                        } else {
                            "ALTERNATIVE"
                        }
                        .into(),
                        detail: format!("{} probability", percent(*p)),
                        active,
                    });
                }
                if shown.is_empty() {
                    result.details.push("Probabilities not captured".into());
                }
            }
            "noul" => {
                if let Some(p) = answer.get("noul").and_then(Value::as_f64) {
                    result.headline = Some(percent(p));
                    result.caption = Some("probability of true".into());
                    result.likelihood = Some(p);
                    outputs.push(Output {
                        title: "True".into(),
                        eyebrow: "P(TRUE)".into(),
                        detail: format!("{} probability", percent(p)),
                        active: true,
                    });
                    outputs.push(Output {
                        title: "False".into(),
                        eyebrow: "P(FALSE)".into(),
                        detail: format!("{} probability · 1 − P(True)", percent(1.0 - p)),
                        active: true,
                    });
                } else {
                    result.details.push("Probability not captured".into());
                }
            }
            "score" => {
                result.probabilities = shown.clone();
                if let Some(score) = answer.get("score").and_then(Value::as_f64) {
                    result.details.push(format!("Score {score:.2}"));
                    outputs.push(Output {
                        title: format!("Score {score}"),
                        eyebrow: "RETURNED SCORE".into(),
                        detail: "Returned by Jev".into(),
                        active: true,
                    });
                } else {
                    result.details.push("Score not captured".into());
                }
            }
            "box" => {
                result.caption = Some(
                    "Image preview unavailable: the relay does not retain image bytes.".into(),
                );
                if let Some(bounds) = answer.get("box") {
                    for key in ["left", "top", "right", "bottom"] {
                        if let Some(v) = bounds.get(key).and_then(Value::as_f64) {
                            result.details.push(format!("{key}  {v:.2}"));
                        }
                    }
                    outputs.push(Output {
                        title: "Region coordinates".into(),
                        eyebrow: "RETURNED BOX".into(),
                        detail: "Your application decides how to use this region.".into(),
                        active: true,
                    });
                }
            }
            _ => result
                .details
                .push("Unknown answer type. Inspect the Response tab for captured JSON.".into()),
        }
        NodeBody::Result(result)
    } else if let Some(error) = response.and_then(|v| v.get("error")) {
        NodeBody::Json(Some(error.clone()))
    } else {
        NodeBody::Text(
            if detail.summary.path == "/v1/models" {
                "This endpoint lists models; it has no question diagram."
            } else {
                "No answer received in this capture."
            }
            .into(),
        )
    };
    // The same content coordinates and vertical spacing used by TraceDiagram.tsx.
    let count = shown.len() as f32;
    let result_height = 280.0_f32.max(
        if compact { 230.0 } else { 166.0 }
            + count * 44.0
            + if kind == "score" { 50.0 } else { 0.0 },
    );
    let spacing = if compact { 112.0 } else { 136.0 };
    let height = 520.0_f32
        .max(result_height + 90.0)
        .max(outputs.len() as f32 * spacing + 60.0);
    let result_y = if compact {
        40.0
    } else {
        40.0_f32.max((height - result_height) / 2.0)
    };
    let mut graph = Graph {
        nodes: vec![
            Node::new(
                "state",
                "Shared context",
                [20.0, 32.0, 240.0, 180.0],
                NodeBody::Json(state.cloned()),
            )
            .kind("STATE"),
            Node::new(
                "question",
                format!("Question · {kind}"),
                [20.0, 288.0, 240.0, 180.0],
                NodeBody::Json(question.and_then(|q| q.get("instructions")).cloned()),
            )
            .kind("INSTRUCTIONS"),
            Node::new(
                "answer",
                name,
                [390.0, result_y, 320.0, result_height],
                body,
            )
            .kind(format!("JEV · {}", kind.to_uppercase()))
            .status(status),
        ],
        edges: vec![
            Edge::new("state", "answer")
                .active(state.is_some())
                .label("state"),
            Edge::new("question", "answer")
                .active(question.is_some())
                .label("question"),
        ],
    };
    for (i, output) in outputs.into_iter().enumerate() {
        let id = format!("output-{i}");
        graph.nodes.push(
            Node::new(
                &id,
                output.title,
                [870.0, 40.0 + i as f32 * spacing, 230.0, 104.0],
                NodeBody::Text(output.detail),
            )
            .kind(output.eyebrow)
            .status(if output.active {
                NodeStatus::Success
            } else {
                NodeStatus::Alternative
            }),
        );
        graph
            .edges
            .push(Edge::new("answer", id).active(output.active));
    }
    (graph, remaining)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graphs_match_original_choice_layout_without_inventing_routes() {
        let mut exchange = crate::demo::exchange(12);
        let questions = exchange.request.json.as_ref().unwrap()["questions"]
            .as_object()
            .unwrap();
        let (graph, _) = graph(&exchange, questions.get_key_value("department"), 0);
        assert_eq!(graph.nodes.len(), 6);
        assert!(graph.validate().is_ok());
        assert_eq!(graph.nodes[0].rect, [20.0, 32.0, 240.0, 180.0]);
        assert_eq!(graph.nodes[3].kind, "RETURNED CHOICE");
        assert_eq!(graph.nodes[4].status, NodeStatus::Alternative);
        assert!(graph.edges[2].active);
        assert!(!graph.edges[3].active);
        match &graph.nodes[2].body {
            NodeBody::Result(r) => {
                assert_eq!(r.selected.as_deref(), Some("billing"));
                assert_eq!(r.probabilities[0].1, 0.94);
            }
            _ => panic!("expected result"),
        }
        exchange.request.json = None;
        exchange.response.json = None;
        let (graph, _) = super::graph(&exchange, None, 0);
        assert!(matches!(graph.nodes[0].body, NodeBody::Json(None)));
        assert_eq!(graph.nodes[2].status, NodeStatus::Unavailable);
        assert!(graph.edges.iter().all(|edge| !edge.active));
    }
    #[test]
    fn noul_complement_is_labeled_and_not_a_selected_boolean() {
        let exchange = crate::demo::exchange(12);
        let questions = exchange.request.json.as_ref().unwrap()["questions"]
            .as_object()
            .unwrap();
        let (graph, _) = graph(&exchange, questions.get_key_value("needs_follow_up"), 0);
        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.nodes[3].kind, "P(TRUE)");
        assert_eq!(graph.nodes[4].kind, "P(FALSE)");
        assert!(matches!(&graph.nodes[4].body,NodeBody::Text(t) if t.contains("1 − P(True)")));
        assert!(graph.edges.iter().all(|e| e.active));
    }
    #[test]
    fn compact_choices_always_include_the_returned_category() {
        let mut exchange = crate::demo::exchange(12);
        let answer = &mut exchange.response.json.as_mut().unwrap()["answers"]["department"];
        answer["choice"] = Value::String("last".into());
        answer["probabilities"] =
            serde_json::json!({"a":0.5,"b":0.2,"c":0.1,"d":0.08,"e":0.06,"f":0.04,"last":0.02});
        let questions = exchange.request.json.as_ref().unwrap()["questions"]
            .as_object()
            .unwrap();
        let (g, remaining) = graph(&exchange, questions.get_key_value("department"), 0);
        assert_eq!(remaining, 3);
        assert!(
            g.nodes
                .iter()
                .any(|n| n.title == "last" && n.kind == "RETURNED CHOICE")
        );
        let (g, remaining) = graph(&exchange, questions.get_key_value("department"), 3);
        assert_eq!(remaining, 0);
        assert_eq!(g.nodes.len(), 10);
    }
}
