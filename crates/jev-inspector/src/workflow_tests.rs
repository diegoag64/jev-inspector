use super::*;

fn runtime(fixture: &str, dark: bool, scale: f32) -> rune::Runtime<State> {
    let mut s = State::default();
    demo::populate(&mut s);
    demo::scenario(&mut s, fixture).unwrap();
    s.dark = dark;
    let mut rt = rune::Runtime::new(
        s,
        view,
        (1100.0, 700.0),
        rune::Env {
            system_dark: dark,
            scale,
        },
    );
    rt.on_key = Some(workspace::on_key);
    settle(&mut rt);
    rt
}
fn settle(rt: &mut rune::Runtime<State>) {
    for _ in 0..4 {
        rt.frame(0.3);
    }
}
fn tap(rt: &mut rune::Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("Missing {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    settle(rt);
}

#[test]
fn incomplete_queries_and_excluded_results_keep_the_investigation() {
    let mut rt = runtime("compare-unset", true, 1.0);
    rt.update(|s, _| s.select(4));
    for query in ["status:", "status:4", "\"unfinished phrase"] {
        let previous = rt.state.visible.iter().map(|s| s.id).collect::<Vec<_>>();
        rt.update(|s, _| {
            s.filter = TextEdit::new(query);
            s.filter_rows();
        });
        settle(&mut rt);
        assert!(rt.state.workspace.query_error.is_some());
        assert_eq!(
            rt.state.visible.iter().map(|s| s.id).collect::<Vec<_>>(),
            previous
        );
        assert_eq!(rt.state.selected, Some(4));
        assert_eq!(rt.state.detail.as_ref().unwrap().summary.id, 4);
    }
    rt.update(|s, _| {
        s.filter = TextEdit::new("status:429");
        s.filter_rows();
    });
    settle(&mut rt);
    assert_eq!(rt.state.visible.len(), 1);
    assert_eq!(rt.state.selected, Some(4));
    assert!(rt.find("show-selected").is_some());
    tap(&mut rt, "show-selected");
    assert_eq!(rt.state.selected, Some(4));
    assert_eq!(rt.state.visible.len(), 12);
    rt.update(|s, _| {
        let mut snapshot = s.snapshot.clone();
        snapshot.instance_id = "restarted".into();
        s.apply_snapshot(snapshot);
    });
    assert!(
        rt.state
            .workspace
            .notice
            .as_ref()
            .unwrap()
            .0
            .contains("Relay restarted")
    );
    rt.update(|s, _| {
        s.filter = TextEdit::new("status:");
        s.filter_rows();
        s.clear_view();
    });
    assert!(rt.state.visible.is_empty());
    assert!(rt.state.selected.is_none());
    rt.update(|s, _| {
        s.workspace.clear_before = 0;
        s.filter_rows();
    });
    assert_eq!(rt.state.visible.len(), 12);
}

#[test]
fn historical_pending_and_disconnected_pending_never_show_live_progress() {
    for (fixture, message) in [
        ("saved-pending", "Incomplete when saved"),
        ("saved-partial", "Incomplete when saved"),
        ("disconnected-pending", "Response status unknown"),
    ] {
        let rt = runtime(fixture, true, 1.0);
        let tree = rt.accessibility_json().to_string();
        assert!(tree.contains(message), "{fixture}");
        assert!(!tree.contains("Waiting for response") && !tree.contains("while you wait"));
        assert!(
            rt.elements().iter().all(|e| e.loop_phase.is_none()),
            "{fixture} must be static"
        );
    }
}

#[test]
fn summary_only_keeps_metadata_when_the_list_is_hidden() {
    let mut rt = runtime("summary-only", true, 1.0);
    rt.update(|s, _| s.workspace.detail_expanded = true);
    settle(&mut rt);
    let summary = rt
        .state
        .snapshot
        .requests
        .iter()
        .find(|s| Some(s.id) == rt.state.selected)
        .unwrap();
    let tree = rt.accessibility_json().to_string();
    for value in [
        summary.request_id.clone().unwrap(),
        ui::timestamp(summary.started_at_ms),
        ui::bytes(summary.request_bytes),
        ui::bytes(summary.response_bytes),
        "Upstream headers".to_uppercase(),
    ] {
        assert!(tree.contains(&value), "Missing {value}");
    }
    assert!(rt.find("requests").is_none());
    assert!(rt.find("detail-loading").is_none());
}

#[test]
fn question_menu_preserves_canvas_and_reaches_every_question() {
    for count in [6, 12, 24] {
        for dark in [false, true] {
            for scale in [1.0, 1.5] {
                let mut rt = runtime(&format!("many-questions-{count}"), dark, scale);
                let check = |rt: &rune::Runtime<State>| {
                    let (r, _) = rt.presented("request-diagram:viewport").unwrap();
                    assert!(r[3] >= 240.0 && r[1] + r[3] < 700.0, "{count}: {r:?}");
                    assert!(rt.find("diagram-expand").is_some());
                };
                check(&rt);
                tap(&mut rt, "diagram-question-trigger");
                assert!(rt.state.diagram_questions_open);
                assert!(rt.presented("diagram-question-content").unwrap().0[2] >= 480.0);
                rt.key(rune::KeyEvent {
                    key: rune::Key::End,
                    mods: Default::default(),
                });
                settle(&mut rt);
                rt.key(rune::KeyEvent {
                    key: rune::Key::Enter,
                    mods: Default::default(),
                });
                settle(&mut rt);
                assert_eq!(
                    rt.state.diagram_question,
                    format!(
                        "question_{:02}_customer_support_triage_and_resolution",
                        count - 1
                    )
                );
                assert!(!rt.state.diagram_questions_open);
                check(&rt);
                rt.update(|s, _| {
                    s.detail.as_mut().unwrap().request.notes =
                        vec!["Long capture note ".repeat(50); 10];
                });
                settle(&mut rt);
                check(&rt);
                tap(&mut rt, "diagram-expand");
                check(&rt);
            }
        }
    }
}

#[test]
fn overview_discloses_probability_limits_and_keeps_returned_choice() {
    let mut rt = runtime("probabilities-40", true, 1.0);
    let tree = rt.accessibility_json().to_string();
    assert!(tree.contains("category_39"));
    assert!(tree.contains("Showing 13 of 40 probabilities"));
    rt.update(|_, tx| tx.scroll_into_view("probabilities-more-department"));
    settle(&mut rt);
    tap(&mut rt, "probabilities-more-department");
    let tree = rt.accessibility_json().to_string();
    assert!(tree.contains("Showing 25 of 40 probabilities"));
    assert!(tree.contains("category_23"));
    assert!(tree.contains("category_39"));
}

#[test]
fn first_use_and_disconnection_show_actionable_setup() {
    for fixture in ["first-use", "setup-disconnected"] {
        let rt = runtime(fixture, true, 1.0);
        let tree = rt.accessibility_json().to_string();
        for text in [
            "cargo run -p jev-bridge",
            "--capture-bodies",
            "TYPESAFE_BASE_URL",
            "Do not append /v1",
            "Connection file:",
        ] {
            assert!(tree.contains(text), "{fixture}: {text}");
        }
    }
}
