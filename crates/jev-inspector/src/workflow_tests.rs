use super::*;

#[test]
fn compact_diagram_help_opens_relevant_instructions_and_returns_focus() {
    let mut rt = runtime("example-support-routing", true, 1.0);
    tap(&mut rt, "diagram-help");
    assert!(rt.state.workspace.help);
    assert!(
        rt.accessibility_json()
            .to_string()
            .contains("Canvas arrow keys pan")
    );
    rt.key(rune::KeyEvent {
        key: rune::Key::Escape,
        mods: Default::default(),
    });
    settle(&mut rt);
    assert!(!rt.state.workspace.help);
    assert!(
        a11y_node(&rt.accessibility_json(), "diagram-help").unwrap()["states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s == "focused")
    );
}

#[test]
fn excluded_selection_navigation_enters_filtered_results_in_both_layouts() {
    for height in [700.0, 900.0] {
        for direction in [-1, 1] {
            let mut state = State::default();
            demo::populate(&mut state);
            let mut rt = rune::Runtime::new(
                state,
                view,
                (1100.0, height),
                rune::Env {
                    system_dark: true,
                    scale: 1.0,
                },
            );
            rt.on_key = Some(workspace::on_key);
            settle(&mut rt);
            rt.update(|s, _| {
                s.errors_only = true;
                s.filter_rows();
            });
            settle(&mut rt);
            assert_eq!(rt.state.selected, Some(12));
            let expected = if direction < 0 { 4 } else { 7 };
            tap(
                &mut rt,
                if direction < 0 {
                    "previous-request"
                } else {
                    "next-request"
                },
            );
            assert_eq!(rt.state.selected, Some(expected));
            rt.update(|s, _| s.select(12));
            settle(&mut rt);
            rt.key(rune::KeyEvent {
                key: if direction < 0 {
                    rune::Key::Up
                } else {
                    rune::Key::Down
                },
                mods: rune::Mods {
                    alt: true,
                    ..Default::default()
                },
            });
            settle(&mut rt);
            assert_eq!(rt.state.selected, Some(expected));
        }
    }
}

#[test]
fn restart_commands_preserve_custom_upstreams_and_history() {
    for upstream in [
        "http://127.0.0.1:8901",
        "https://custom.example/api",
        "http://localhost:8901/path?literal=$value",
    ] {
        let mut s = State::default();
        demo::scenario(&mut s, "first-use").unwrap();
        s.snapshot.upstream = upstream.into();
        s.snapshot.history_limit = 150;
        let command = setup_view::commands(&s).0;
        let normalized = reqwest::Url::parse(upstream).unwrap();
        assert!(command.contains(&format!("--upstream '{}'", normalized.as_str())));
        assert!(command.contains("--history 150"));
    }
}

#[test]
fn modal_controls_contain_investigation_shortcuts_and_escape_restores_them() {
    for modal in ["help", "export-menu", "diagram-question-trigger"] {
        for key in ['k', 'f', 'p', 'b', 'l', 'e', 'o', '1', '2', '3', '4', '5'] {
            let mut rt = runtime("shared-prefix-questions", true, 1.0);
            tap(&mut rt, modal);
            let selected = rt.state.selected;
            let tab = rt.state.tab;
            rt.key(rune::KeyEvent {
                key: rune::Key::Command(key),
                mods: rune::Mods {
                    cmd: true,
                    ..Default::default()
                },
            });
            settle(&mut rt);
            assert_eq!(rt.state.selected, selected, "{modal}: {key}");
            assert_eq!(rt.state.tab, tab, "{modal}: {key}");
            assert_eq!(rt.state.workspace.clear_before, 0, "{modal}: {key}");
            assert!(rt.state.workspace.starred.is_empty(), "{modal}: {key}");
            assert!(
                !rt.state.workspace.paused && !rt.state.filter.focused,
                "{modal}: {key}"
            );
            assert!(
                !rt.state.request_text.search_open && !rt.state.response_text.search_open,
                "{modal}: {key}"
            );
            assert!(rt.file_dialog().is_none(), "{modal}: {key}");
            assert_eq!(rt.state.workspace.help, modal == "help");
            assert_eq!(rt.state.workspace.export, modal == "export-menu");
            assert_eq!(
                rt.state.diagram_questions_open,
                modal == "diagram-question-trigger"
            );
            rt.key(rune::KeyEvent {
                key: rune::Key::Escape,
                mods: Default::default(),
            });
            settle(&mut rt);
            assert!(
                !rt.state.workspace.help
                    && !rt.state.workspace.export
                    && !rt.state.diagram_questions_open
            );
            rt.key(rune::KeyEvent {
                key: rune::Key::Command('l'),
                mods: rune::Mods {
                    cmd: true,
                    ..Default::default()
                },
            });
            settle(&mut rt);
            assert!(rt.state.selected.is_none() && rt.state.visible.is_empty());
        }
    }
}

#[test]
fn overview_retains_every_shared_prefix_answer_identity() {
    for dark in [false, true] {
        let mut rt = runtime("shared-prefix-questions", dark, 1.0);
        rt.update(|s, _| s.select_tab(0));
        settle(&mut rt);
        let tree = rt.accessibility_json().to_string();
        for name in rt
            .state
            .detail
            .as_ref()
            .unwrap()
            .response
            .json
            .as_ref()
            .unwrap()["answers"]
            .as_object()
            .unwrap()
            .keys()
        {
            assert!(
                tree.contains(name),
                "Missing original answer identity {name}"
            );
        }
    }
}

#[test]
fn partial_results_keep_snapshot_context_and_connected_setup_starts_with_routing() {
    let mut rt = runtime("saved-partial", true, 1.0);
    rt.update(|s, _| {
        let mut d = s.detail.clone().unwrap();
        d.response.json = demo::exchange(12).response.json;
        s.set_detail(d);
    });
    for tab in [0, 3, 4] {
        rt.update(|s, _| s.select_tab(tab));
        settle(&mut rt);
        let note = a11y_node(&rt.accessibility_json(), "partial-response-note").unwrap();
        assert!(note["name"].as_str().unwrap().contains("will not update"));
        if tab == 0 {
            let scroll = a11y_node(&rt.accessibility_json(), "detail-scroll").unwrap();
            assert!(
                scroll["scroll"][2].as_f64().unwrap() > 50.0,
                "Partial overview must retain scrolling to all results and capture metadata"
            );
        }
    }
    let rt = runtime("first-use", true, 1.0);
    assert!(rt.state.connected);
    let route = rt.presented("setup-route").unwrap().0;
    let inspector = rt.presented("setup-inspector").unwrap().0;
    let relay = rt.presented("setup-relay").unwrap().0;
    assert!(route[1] < inspector[1] && route[1] < relay[1]);
}

#[test]
fn pending_context_is_consistent_across_overview_headers_filters_and_counts() {
    for (fixture, expected) in [
        ("saved-pending", "Response incomplete when saved"),
        ("saved-partial", "Response incomplete when saved"),
        ("disconnected-pending", "Response status unknown"),
        ("pending-response", "Response pending"),
    ] {
        let mut rt = runtime(fixture, true, 1.0);
        rt.update(|s, _| s.select_tab(0));
        settle(&mut rt);
        let tree = rt.accessibility_json().to_string();
        assert!(tree.contains(expected), "{fixture}");
        assert!(
            !tree.contains("No typed answers in this response"),
            "{fixture}"
        );
        if fixture != "pending-response" {
            assert!(
                !tree.contains("In flight") && !tree.contains("IN FLIGHT"),
                "{fixture}"
            );
        }
        if fixture == "disconnected-pending" {
            assert!(
                tree.contains("LAST KNOWN PENDING")
                    && tree.contains("Pending (offline)")
                    && tree.contains("Status unknown")
            );
        }
    }
}

#[test]
fn diagram_disclosure_counts_only_categories_added_by_the_next_click() {
    for count in [7, 8, 10, 121] {
        for rank in [0, 2, 3, 4, count - 1] {
            let mut rt = runtime("compare-unset", true, 1.0);
            rt.update(|s,_| {
                let mut d = demo::exchange(12);
                let values: serde_json::Map<String, serde_json::Value> = (0..count).map(|i| (format!("c{i:03}"), serde_json::json!((count-i) as f64 / (count*count) as f64))).collect();
                d.response.json = Some(serde_json::json!({"answers":{"department":{"type":"choice","choice":format!("c{rank:03}"),"probabilities":values}}}));
                s.set_detail(d);
                s.diagram_question = "department".into();
                s.select_tab(3);
            });
            settle(&mut rt);
            for _ in 0..3 {
                let Some(button) = a11y_node(&rt.accessibility_json(), "diagram-more-categories")
                else {
                    break;
                };
                let count_nodes = |s: &State| {
                    let d = s.detail.as_ref().unwrap();
                    let q = d.request.json.as_ref().unwrap()["questions"]
                        .as_object()
                        .unwrap()
                        .get_key_value("department");
                    let extra = s.diagram.content.visible("categories").saturating_sub(12);
                    diagram::graph(d, q, extra)
                        .0
                        .nodes
                        .iter()
                        .filter(|n| n.id.starts_with("output-"))
                        .count()
                };
                let before = count_nodes(&rt.state);
                tap(&mut rt, "diagram-more-categories");
                let added = count_nodes(&rt.state) - before;
                assert!(
                    button["name"].as_str().unwrap().starts_with(&format!(
                        "Show {added} more {}",
                        if added == 1 { "category" } else { "categories" }
                    )),
                    "count={count} rank={rank}: {button}"
                );
            }
        }
    }
}

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

fn a11y_node(tree: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
    if tree["key"].as_str() == Some(key) {
        return Some(tree.clone());
    }
    tree["children"]
        .as_array()?
        .iter()
        .find_map(|child| a11y_node(child, key))
}

#[test]
fn changed_find_documents_reveal_once_and_respect_manual_scrolling() {
    for reduced in [false, true] {
        let mut rt = runtime("long-json", true, 1.0);
        rt.set_system_reduced_motion(reduced);
        rt.update(|s, tx| {
            let mut other = s.detail.clone().unwrap();
            other.summary.id = 11;
            other.response.json = Some(serde_json::json!({"rows": (0..1500).map(|i| {
                if i == 800 { "needle-雪😀".into() } else { format!("{i}-{}", "雪😀".repeat(12)) }
            }).collect::<Vec<String>>() }));
            s.workspace.cache.insert(11, other.clone());
            other.summary.id = 12;
            other.response.json = Some(serde_json::json!({"rows": (0..1500).map(|i| {
                if i == 1490 { "needle-雪😀".into() } else { format!("{i}-{}", "雪😀".repeat(12)) }
            }).collect::<Vec<String>>() }));
            s.set_detail(other);
            s.tab = 2;
            s.response_text.search_open = true;
            s.response_text.search = TextEdit::new("needle-雪😀");
            s.response_text.find("response-json", 0, tx);
        });
        settle(&mut rt);
        rt.update(|s, _| s.select(11));
        settle(&mut rt);
        let scroll =
            a11y_node(&rt.accessibility_json(), "response-json:scroll").unwrap()["scroll"].clone();
        assert!(scroll[1].as_f64().unwrap() > 300.0, "{scroll}");
        rt.update(|_, tx| tx.scroll_to_y("response-json:scroll", 0.0));
        settle(&mut rt);
        let scroll =
            a11y_node(&rt.accessibility_json(), "response-json:scroll").unwrap()["scroll"].clone();
        assert_eq!(scroll[1], 0.0, "manual scrolling must not be overridden");
        rt.update(|s, _| s.select(12));
        settle(&mut rt);
        let scroll =
            a11y_node(&rt.accessibility_json(), "response-json:scroll").unwrap()["scroll"].clone();
        assert!(scroll[1].as_f64().unwrap() > 300.0);
        tap(&mut rt, "response-json:tree");
        tap(&mut rt, "response-json:find");
        let scroll =
            a11y_node(&rt.accessibility_json(), "response-json:scroll").unwrap()["scroll"].clone();
        assert!(
            scroll[1].as_f64().unwrap() > 300.0,
            "Tree to Find must reveal the retained match"
        );
        tap(&mut rt, "tab-0");
        tap(&mut rt, "tab-2");
        let scroll =
            a11y_node(&rt.accessibility_json(), "response-json:scroll").unwrap()["scroll"].clone();
        assert!(
            scroll[1].as_f64().unwrap() > 300.0,
            "returning to Response must reveal the retained match"
        );
    }
}

#[test]
fn retained_find_reveals_horizontal_matches_and_handles_no_match_replacement() {
    let mut rt = runtime("long-json", true, 1.0);
    rt.update(|s, tx| {
        let mut d = s.detail.clone().unwrap();
        d.summary.id = 11;
        d.response.json = Some(serde_json::json!({"line": format!("{}needle", "x".repeat(2000))}));
        s.workspace.cache.insert(11, d.clone());
        d.summary.id = 12;
        d.response.json = Some(serde_json::json!({"line": format!("{}needle", "x".repeat(4000))}));
        s.set_detail(d);
        s.tab = 2;
        s.response_text.search_open = true;
        s.response_text.search = TextEdit::new("needle");
        s.response_text.find("response-json", 0, tx);
    });
    settle(&mut rt);
    rt.update(|s, _| s.select(11));
    settle(&mut rt);
    let scroll =
        a11y_node(&rt.accessibility_json(), "response-json:scroll").unwrap()["scroll"].clone();
    assert!(scroll[0].as_f64().unwrap() > 10_000.0);
    rt.update(|s, _| s.select(10));
    settle(&mut rt);
    assert!(rt.accessibility_json().to_string().contains("0 matches"));
}

#[test]
fn unicode_find_places_the_shaped_match_inside_the_viewport() {
    for prefix in [
        "x",
        "\u{754c}",
        "\u{1f680}",
        "e\u{301}",
        "x\u{754c}\u{1f680}e\u{301}",
    ] {
        for (dark, scale) in [(false, 1.0), (true, 1.5)] {
            let mut rt = runtime("long-json", dark, scale);
            let value = serde_json::json!({"line": format!("{}NEEDLE", prefix.repeat(1000))});
            let source = serde_json::to_string_pretty(&value).unwrap();
            // These cases stay inside one preview window; the tail-window test covers paging.
            assert!(source.len() < 16_000);
            let before = source
                .split("NEEDLE")
                .next()
                .unwrap()
                .rsplit('\n')
                .next()
                .unwrap();
            rt.update(|s, tx| {
                let mut d = s.detail.clone().unwrap();
                d.response.json = Some(value);
                s.set_detail(d);
                s.response_text.search_open = true;
                s.response_text.search = TextEdit::new("NEEDLE");
                s.response_text.find("response-json", 0, tx);
            });
            settle(&mut rt);
            let code = rt.presented("response-json:code").unwrap().0;
            let viewport = rt.presented("response-json:scroll").unwrap().0;
            let font = rune::TextStyle::new(13.0, 400.0).mono();
            let left = code[0] + rune::text::advance(before, font);
            let right = left + rune::text::advance("NEEDLE", font);
            assert!(
                left >= viewport[0] && right <= viewport[0] + viewport[2],
                "{prefix:?} at {scale}: match {left}..{right}, viewport {viewport:?}"
            );
        }
    }
}

#[test]
fn overview_pending_labels_respect_saved_and_disconnected_context() {
    for fixture in ["saved-pending", "saved-partial", "disconnected-pending"] {
        let mut rt = runtime(fixture, true, 1.0);
        rt.update(|s, _| {
            let mut d = s.detail.clone().unwrap();
            d.summary.upstream_headers_ms = Some(12.34);
            s.set_detail(d);
            s.select_tab(0);
        });
        settle(&mut rt);
        let tree = a11y_node(&rt.accessibility_json(), "detail-scroll")
            .unwrap()
            .to_string();
        assert!(
            tree.contains("12.34 ms"),
            "{fixture}: preserve recorded headers"
        );
        assert!(
            !tree.contains("In flight") && !tree.contains("as the exchange progresses"),
            "{fixture}"
        );
        assert!(tree.contains(if fixture == "disconnected-pending" {
            "Status unknown"
        } else {
            "Incomplete when saved"
        }));
    }
    let mut rt = runtime("summary-only", true, 1.0);
    rt.update(|s, _| {
        s.workspace.session_name = Some("incomplete.jev.json".into());
        let selected = s.selected;
        let r = s
            .snapshot
            .requests
            .iter_mut()
            .find(|r| Some(r.id) == selected)
            .unwrap();
        r.outcome = Outcome::Pending;
        r.duration_ms = None;
    });
    settle(&mut rt);
    let tree = rt.accessibility_json().to_string();
    assert!(!tree.contains("In flight"));
    assert!(tree.contains("Not completed"));
}

#[test]
fn setup_commands_share_custom_paths_and_listener_addresses() {
    let mut s = State {
        connection_file: "artifacts/custom path/O'Brien/$session.json".into(),
        ..State::default()
    };
    for listen in ["127.0.0.1:9876", "127.0.0.1:8788", "[::1]:9876"] {
        s.snapshot.relay_url = format!("http://{listen}");
        let (bridge, inspector, backend, relay) = setup_view::commands(&s);
        assert!(bridge.contains("--connection-file"));
        assert!(bridge.contains(listen) && backend.contains(listen));
        assert!(bridge.contains("--inspector 127.0.0.1:0"));
        assert_eq!(relay, s.snapshot.relay_url);
        let quoted_path = inspector.split("--connection-file ").nth(1).unwrap();
        assert!(bridge.contains(quoted_path));
        if cfg!(windows) {
            assert!(quoted_path.contains("O''Brien/$session.json"));
        }
    }
}

#[test]
fn probability_button_counts_actual_additions_and_retains_disclosure() {
    for total in [13, 14, 24, 25, 26, 40] {
        for returned in [0, 11, 12, total - 1] {
            let mut rt = runtime("probabilities-40", true, 1.0);
            rt.update(|s, _| {
                let mut detail = s.detail.clone().unwrap();
                let answer = &mut detail.response.json.as_mut().unwrap()["answers"]["department"];
                answer["choice"] = serde_json::json!(format!("category_{returned:02}"));
                answer["probabilities"] = serde_json::Value::Object(
                    (0..total)
                        .map(|i| {
                            (
                                format!("category_{i:02}"),
                                serde_json::json!(1.0 / f64::from(total)),
                            )
                        })
                        .collect(),
                );
                s.set_detail(detail);
            });
            settle(&mut rt);
            let old: std::collections::BTreeSet<_> =
                (0..total).filter(|i| *i < 12 || *i == returned).collect();
            let next: std::collections::BTreeSet<_> =
                (0..total).filter(|i| *i < 24 || *i == returned).collect();
            if old.len() == total as usize {
                continue;
            }
            let expected = format!("Show {} more probabilities", next.difference(&old).count());
            let node =
                a11y_node(&rt.accessibility_json(), "probabilities-more-department").unwrap();
            assert_eq!(node["name"], expected);
            rt.update(|_, tx| tx.scroll_into_view("probabilities-more-department"));
            settle(&mut rt);
            tap(&mut rt, "probabilities-more-department");
            let tree = rt.accessibility_json().to_string();
            for i in &next {
                assert!(tree.contains(&format!("category_{i:02}")));
            }
            rt.update(|s, _| s.select(11));
            settle(&mut rt);
            rt.update(|s, _| s.select(12));
            settle(&mut rt);
            assert_eq!(
                rt.state
                    .workspace
                    .probability_limits
                    .get(&(12, "department".into())),
                Some(&24)
            );
        }
    }
}

#[test]
fn tail_comparison_opens_current_and_copies_baseline_after_eviction() {
    let mut rt = runtime("compare-tail", true, 1.0);
    let baseline = rt.state.workspace.comparison.baseline.clone().unwrap();
    rt.update(|s, _| {
        s.snapshot.requests.retain(|r| r.id != baseline.summary.id);
        s.workspace.cache.remove(&baseline.summary.id);
    });
    settle(&mut rt);
    tap(&mut rt, "compare-copy-response.json");
    let copied = rt.copy_request.take().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(copied.content().text()).unwrap(),
        baseline.response.json.unwrap()
    );
    assert_eq!(rt.state.selected, Some(12));
    tap(&mut rt, "compare-open-response.json");
    assert_eq!(rt.state.tab, 2);
    assert_eq!(rt.state.selected, Some(12));
}
