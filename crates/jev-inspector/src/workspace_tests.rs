use super::*;

fn runtime(size: (f32, f32)) -> rune::Runtime<State> {
    let mut s = State::default();
    demo::populate(&mut s);
    let mut rt = rune::Runtime::new(
        s,
        view,
        size,
        rune::Env {
            system_dark: true,
            scale: 1.0,
        },
    );
    rt.on_key = Some(workspace::on_key);
    rt.frame(0.0);
    rt
}

#[test]
fn request_scrollbar_drags_the_table_and_hides_when_rows_fit() {
    let mut rt = runtime((1100.0, 700.0));
    // Compact rows now fit this fixture; use roomy rows to exercise overflow.
    rt.update(|s, _| s.workspace.compact = false);
    rt.frame(0.2);
    let selected = rt.state.selected;
    let (x, y) = rt.find("request-scroll-thumb").expect("overflow scrollbar");
    rt.pointer_down(x, y);
    rt.pointer_move(x, y + 80.0);
    rt.pointer_up(x, y + 80.0);
    rt.frame(0.4);
    rt.frame(0.5);
    let (_, after) = rt.find("request-scroll-thumb").unwrap();
    assert!(
        after > y + 20.0,
        "thumb follows the scrolled viewport: {y} -> {after}"
    );
    assert_eq!(rt.state.selected, selected, "scrolling preserves selection");
    rt.update(|s, _| {
        s.filter = TextEdit::new("429");
        s.filter_rows();
    });
    rt.frame(0.6);
    rt.frame(0.8);
    assert!(
        rt.find("request-scroll-thumb").is_none(),
        "no thumb without overflow"
    );
}

#[test]
fn follow_returns_to_top_on_enable_and_arrivals_but_leaves_manual_browsing_alone() {
    let mut rt = runtime((1100.0, 700.0));
    rt.poll = Some(poll);
    rt.update(|s, _| {
        s.workspace.compact = false;
        s.demo = false;
    });
    rt.frame(0.2);
    let scroll_down = |rt: &mut rune::Runtime<State>| {
        rt.update(|_, tx| tx.scroll_to_instant("requests/viewport", 0.0, 180.0));
        rt.frame(0.2);
        rt.frame(0.2);
    };
    let offset = |rt: &rune::Runtime<State>| {
        let elements = rt.elements();
        elements
            .iter()
            .find(|e| e.path.ends_with("requests/viewport/rows"))
            .expect("request rows")
            .scrolled[1]
    };
    scroll_down(&mut rt);
    assert!(offset(&rt) > 100.0);
    tap(&mut rt, "follow");
    rt.frame(0.2);
    assert!(
        offset(&rt).abs() < 0.5,
        "enabling Follow scrolls to the top"
    );
    scroll_down(&mut rt);
    rt.update(|s, _| {
        let mut snapshot = s.snapshot.clone();
        snapshot.requests.insert(0, demo::exchange(13).summary);
        s.apply_snapshot(snapshot);
    });
    rt.frame(0.2);
    rt.frame(0.2);
    assert_eq!(rt.state.selected, Some(13));
    assert!(
        offset(&rt).abs() < 0.5,
        "new rows stay at the top while following"
    );
    tap(&mut rt, "follow");
    scroll_down(&mut rt);
    rt.update(|s, _| {
        let mut snapshot = s.snapshot.clone();
        snapshot.requests.insert(0, demo::exchange(14).summary);
        s.apply_snapshot(snapshot);
    });
    rt.frame(0.2);
    rt.frame(0.2);
    assert!(
        offset(&rt) > 100.0,
        "arrivals must not reset manual scrolling"
    );
    assert_eq!(rt.state.selected, Some(13));
}

#[test]
fn new_request_rows_start_in_opposite_theme_then_settle_in_both_densities() {
    for dark in [true, false] {
        for compact in [true, false] {
            for reduced in [false, true] {
                let mut rt = runtime((1100.0, 700.0));
                rt.set_system_reduced_motion(reduced);
                rt.update(|s, _| {
                    s.dark = dark;
                    s.workspace.compact = compact;
                });
                rt.frame(0.3);
                rt.update(|s, _| {
                    s.workspace.arrivals.tick(10.0);
                    let mut snapshot = s.snapshot.clone();
                    snapshot.requests.insert(0, demo::exchange(13).summary);
                    s.apply_snapshot(snapshot);
                    s.select(13);
                });
                rt.frame(0.0);
                let row_color = |rt: &rune::Runtime<State>| {
                    rt.elements()
                        .iter()
                        .find(|e| e.path.ends_with("/rows/13"))
                        .expect("new row mounted")
                        .fill
                };
                let expected =
                    motion::traffic_selected(&ui::kit(if reduced { dark } else { !dark }));
                assert_eq!(
                    row_color(&rt),
                    expected,
                    "dark={dark}, compact={compact}, reduced={reduced}"
                );
                let elements = rt.elements();
                let text = elements
                    .iter()
                    .find(|e| {
                        e.path.contains("/rows/13/request/")
                            && e.text.as_ref().is_some_and(|t| {
                                if compact {
                                    t != "#013"
                                } else {
                                    t.contains("POST")
                                }
                            })
                    })
                    .expect("request text");
                assert_eq!(
                    text.text_color,
                    Some(ui::kit(if reduced { dark } else { !dark }).c.text_base)
                );
                rt.update(|s, _| {
                    s.workspace.arrivals.tick(10.6);
                });
                rt.frame(0.0);
                if !reduced {
                    assert_ne!(
                        row_color(&rt),
                        expected,
                        "row fades without another snapshot"
                    );
                }
                rt.update(|s, _| {
                    s.workspace.arrivals.tick(11.3);
                });
                rt.frame(0.3);
                assert_eq!(row_color(&rt), motion::traffic_selected(&ui::kit(dark)));
            }
        }
    }
}
fn tap(rt: &mut rune::Runtime<State>, key: &str) {
    let (x, y) = rt
        .find(key)
        .unwrap_or_else(|| panic!("missing control {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.3);
}
fn command(rt: &mut rune::Runtime<State>, c: char) {
    rt.key(rune::KeyEvent {
        key: rune::Key::Command(c),
        mods: rune::Mods {
            cmd: true,
            ..Default::default()
        },
    });
    rt.frame(0.02);
}
fn wait_job(rt: &mut rune::Runtime<State>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while rt.state.workspace.job.is_some() {
        assert!(
            std::time::Instant::now() < deadline,
            "background I/O failed to finish"
        );
        rt.state.poll_job();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    rt.frame(0.02);
}

fn accessible<'a>(node: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if node["key"] == key {
        return Some(node);
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|n| accessible(n, key))
}

fn focused_key(node: &serde_json::Value) -> Option<String> {
    if node["states"]
        .as_array()
        .is_some_and(|s| s.contains(&serde_json::json!("focused")))
    {
        return node["key"].as_str().map(str::to_string);
    }
    node["children"].as_array()?.iter().find_map(focused_key)
}

#[test]
fn tab_order_reaches_primary_controls_and_help_contains_focus() {
    let mut rt = runtime((1100.0, 700.0));
    let mut reached = std::collections::BTreeSet::new();
    for _ in 0..70 {
        rt.key(rune::KeyEvent {
            key: rune::Key::Tab,
            mods: Default::default(),
        });
        rt.frame(0.02);
        let tree = rt.accessibility_json();
        if let Some(key) = focused_key(&tree) {
            reached.insert(key);
        }
        if accessible(&tree, "request-filter")
            .and_then(focused_key)
            .is_some()
        {
            reached.insert("request-filter".into());
        }
    }
    for key in [
        "open-session",
        "export-menu",
        "follow",
        "clear-view",
        "request-filter",
        "splitter",
        "tab-0",
        "star-exchange",
    ] {
        assert!(
            reached.contains(key),
            "{key} absent from Tab order: {reached:?}"
        );
    }
    tap(&mut rt, "splitter");
    let width = rt.state.workspace.list_width;
    rt.key(rune::KeyEvent {
        key: rune::Key::Left,
        mods: Default::default(),
    });
    rt.frame(0.02);
    assert_eq!(rt.state.workspace.list_width, width - 20.0);
    tap(&mut rt, "help");
    let mut scrolled_help = false;
    for _ in 0..8 {
        rt.key(rune::KeyEvent {
            key: rune::Key::Tab,
            mods: Default::default(),
        });
        rt.frame(0.02);
        let tree = rt.accessibility_json();
        let dialog = accessible(&tree, "help-dialog").unwrap();
        assert_eq!(
            focused_key(&tree),
            focused_key(dialog),
            "Tab escaped the modal"
        );
        assert!(focused_key(dialog).is_some());
        if focused_key(dialog).as_deref() == Some("viewport") && !scrolled_help {
            rt.key(rune::KeyEvent {
                key: rune::Key::End,
                mods: Default::default(),
            });
            rt.frame(0.3);
            assert_eq!(rt.scroll_at_bottom("help-scroll/viewport"), Some(true));
            scrolled_help = true;
        }
    }
    assert!(scrolled_help, "Help content must be keyboard reachable");
}

#[test]
fn summary_only_and_loading_request_ids_copy_from_the_selected_summary() {
    let mut rt = runtime((1100.0, 700.0));
    for saved in [false, true] {
        rt.state.detail = None;
        rt.state.workspace.cache.clear();
        rt.state.workspace.session_name = saved.then(|| "summary-only.json".into());
        rt.invalidate();
        rt.frame(0.3);
        tap(&mut rt, "copy-id");
        assert_eq!(
            rt.copy_request.take().unwrap().content().text(),
            "demo-request-012"
        );
        assert_eq!(
            rt.state.workspace.notice.as_ref().unwrap().0,
            "Request ID copied"
        );
    }
}

#[test]
fn tabs_report_selection_and_keyboard_focus_returns_after_help() {
    let mut rt = runtime((1100.0, 700.0));
    tap(&mut rt, "tab-0");
    rt.key(rune::KeyEvent {
        key: rune::Key::Right,
        mods: Default::default(),
    });
    rt.key(rune::KeyEvent {
        key: rune::Key::Enter,
        mods: Default::default(),
    });
    rt.frame(0.1);
    assert_eq!(rt.state.tab, 1);
    let tree = rt.accessibility_json();
    assert_eq!(
        accessible(&tree, "detail-tabs").unwrap()["role"],
        "tab-list"
    );
    let tab = accessible(&tree, "tab-1").unwrap();
    assert_eq!(tab["role"], "tab");
    assert!(
        tab["states"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("selected"))
    );
    rt.key(rune::KeyEvent {
        key: rune::Key::Function(1),
        mods: Default::default(),
    });
    rt.frame(0.3);
    assert!(rt.state.workspace.help);
    rt.key(rune::KeyEvent {
        key: rune::Key::Escape,
        mods: Default::default(),
    });
    rt.frame(0.3);
    assert!(!rt.state.workspace.help);
    let tree = rt.accessibility_json();
    assert!(
        accessible(&tree, "tab-1").unwrap()["states"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("focused"))
    );
}

#[test]
fn tree_copy_reports_the_property_and_preserves_its_json() {
    let mut rt = runtime((1100.0, 700.0));
    rt.state.tab = 2;
    rt.state.response_text.tree = true;
    rt.state.response_text.outline.select("$/answers");
    rt.invalidate();
    rt.frame(0.3);
    tap(&mut rt, "copy-json-value");
    let copy = rt.copy_request.take().unwrap();
    let value: serde_json::Value = serde_json::from_str(copy.content().text()).unwrap();
    assert_eq!(
        value,
        rt.state.response_text.value.as_ref().unwrap()["answers"]
    );
    assert!(
        rt.state
            .workspace
            .notice
            .as_ref()
            .unwrap()
            .0
            .contains("$/answers")
    );
}

#[test]
fn pause_follow_and_clear_icons_keep_their_state_and_recovery() {
    let mut rt = runtime((1100.0, 700.0));
    rt.update(|s, _| {
        s.demo = false;
        s.connected = true;
    });
    rt.frame(0.3);
    tap(&mut rt, "pause");
    assert!(rt.state.workspace.paused);
    let tree = rt.accessibility_json();
    let pause = accessible(&tree, "pause").unwrap();
    assert_eq!(pause["name"], "Resume live updates");
    assert!(
        pause["states"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("checked"))
    );
    tap(&mut rt, "pause");
    assert!(!rt.state.workspace.paused);
    tap(&mut rt, "follow");
    assert!(rt.state.workspace.follow);
    tap(&mut rt, "clear-view");
    assert!(rt.state.visible.is_empty());
    tap(&mut rt, "empty-restore");
    assert_eq!(rt.state.visible.len(), 12);
}

#[test]
fn native_dialog_workflow_saves_reopens_and_returns_to_demo() {
    let mut rt = runtime((1440.0, 900.0));
    let path = std::env::temp_dir().join(format!("jev-session-ui-{}.json", std::process::id()));
    tap(&mut rt, "star-exchange");
    assert!(rt.state.workspace.starred.contains(&12));
    tap(&mut rt, "export-menu");
    tap(&mut rt, "export-0");
    assert_eq!(rt.file_dialog().unwrap().0, rune::FileDialogKind::Save);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(0.02);
    wait_job(&mut rt);
    let saved = session::read(&path).unwrap();
    assert_eq!(saved.exchanges.len(), 12);
    assert!(saved.starred.contains(&12));
    assert!(
        rt.state
            .workspace
            .notice
            .as_ref()
            .is_some_and(|(_, error)| !error)
    );
    tap(&mut rt, "open-session");
    assert_eq!(rt.file_dialog().unwrap().0, rune::FileDialogKind::Open);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(0.02);
    wait_job(&mut rt);
    assert!(rt.state.workspace.session_name.is_some());
    assert!(rt.state.detail.is_some());
    assert_eq!(rt.state.snapshot.requests.len(), 12);
    tap(&mut rt, "return-live");
    assert!(rt.state.workspace.session_name.is_none());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn manual_native_fixture_is_valid_and_has_exact_expected_values() {
    let fixture: session::Session = serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-check.jev.json"
    ))
    .unwrap();
    session::validate(&fixture).unwrap();
    assert_eq!(fixture.snapshot.requests.len(), 2);
    assert_eq!(
        fixture.snapshot.requests[0].request_id.as_deref(),
        Some("native-qa-summary-only")
    );
    assert_eq!(fixture.missing, vec![2]);
    assert!(fixture.starred.contains(&1));
    assert_eq!(
        fixture.exchanges[&1].response.json.as_ref().unwrap()["message"],
        "native clipboard check"
    );
}
#[test]
fn cancel_dialog_and_failed_import_keep_the_current_investigation() {
    let mut rt = runtime((1100.0, 700.0));
    command(&mut rt, 'o');
    rt.resolve_file_dialog(None).unwrap();
    rt.frame(0.02);
    assert!(rt.state.workspace.job.is_none());
    assert_eq!(rt.state.selected, Some(12));
    command(&mut rt, 'o');
    rt.resolve_file_dialog(Some(
        std::env::temp_dir().join("jev-nonexistent-session-folder/no-file.json"),
    ))
    .unwrap();
    rt.frame(0.02);
    wait_job(&mut rt);
    assert_eq!(rt.state.selected, Some(12));
    assert!(rt.state.workspace.session_name.is_none());
    assert!(
        rt.state
            .workspace
            .notice
            .as_ref()
            .is_some_and(|(_, error)| *error)
    );
}
#[test]
fn keyboard_search_star_clear_restore_and_json_find_are_wired() {
    let mut rt = runtime((1440.0, 900.0));
    command(&mut rt, 'k');
    rt.key(rune::KeyEvent {
        key: rune::Key::Text("status:429".into()),
        mods: Default::default(),
    });
    rt.frame(0.02);
    assert_eq!(rt.state.visible.len(), 1);
    assert_eq!(
        rt.state.selected,
        Some(12),
        "Filtering retains the investigation"
    );
    rt.update(|s, _| s.select(7));
    rt.frame(0.2);
    assert_eq!(rt.state.selected, Some(7));
    command(&mut rt, 'b');
    assert!(rt.state.workspace.starred.contains(&7));
    command(&mut rt, 'l');
    assert!(rt.state.visible.is_empty());
    tap(&mut rt, "empty-restore");
    assert_eq!(rt.state.visible.len(), 1);
    command(&mut rt, '2');
    command(&mut rt, 'f');
    assert!(rt.state.request_text.search_open);
    rt.key(rune::KeyEvent {
        key: rune::Key::Text("billing".into()),
        mods: Default::default(),
    });
    rt.frame(0.02);
    assert_eq!(rt.state.request_text.search.text, "billing");
    tap(&mut rt, "request-json:tree");
    assert!(rt.state.request_text.tree);
}
#[test]
fn comparison_and_resize_controls_work_at_minimum_size() {
    let mut rt = runtime((1100.0, 700.0));
    tap(&mut rt, "tab-4");
    tap(&mut rt, "set-baseline");
    assert_eq!(
        rt.state
            .workspace
            .comparison
            .baseline
            .as_ref()
            .unwrap()
            .summary
            .id,
        12
    );
    tap(&mut rt, "errors-filter");
    assert_eq!(
        rt.state.selected,
        Some(12),
        "Filtering retains the investigation"
    );
    rt.update(|s, _| s.select(7));
    rt.frame(0.2);
    assert_eq!(rt.state.selected, Some(7));
    assert_eq!(rt.state.workspace.comparison.diff.files().len(), 2);
    assert!(rt.state.workspace.comparison.diff.collapsed(0));
    tap(&mut rt, "diff-mode");
    assert_eq!(
        rt.state.workspace.comparison.diff.mode(),
        rune_kit::diff::DiffMode::Split
    );
    tap(&mut rt, "expand-comparison");
    assert!(rt.state.workspace.detail_expanded);
    assert!(rt.find("requests").is_none());
    tap(&mut rt, "expand-detail");
    assert!(!rt.state.workspace.detail_expanded);
    let (x, y) = rt.find("splitter").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_move(x - 65.0, y);
    rt.pointer_up(x - 65.0, y);
    rt.frame(0.02);
    assert!(rt.state.workspace.list_width < 550.0);
}
#[test]
fn imported_session_ignores_live_updates_and_instance_change_during_pause_is_safe() {
    let mut s = State::default();
    demo::populate(&mut s);
    s.workspace.session_name = Some("saved.json".into());
    let mut update = s.snapshot.clone();
    update.instance_id = "another".into();
    update.total_requests = 1000;
    s.mailbox.lock().unwrap().update = Some(Ok(feed::Update {
        snapshot: update.clone(),
        detail: None,
    }));
    poll(&mut s, 0.0, &mut Tx::default());
    assert_eq!(s.snapshot.total_requests, 12);
    s.workspace.session_name = None;
    s.workspace.paused = true;
    s.mailbox.lock().unwrap().update = Some(Ok(feed::Update {
        snapshot: update,
        detail: Some((12, Ok(demo::exchange(12)))),
    }));
    poll(&mut s, 0.0, &mut Tx::default());
    assert_eq!(s.snapshot.instance_id, "demo");
    s.demo = false;
    s.toggle_pause();
    assert_eq!(s.snapshot.instance_id, "another");
    assert!(s.mailbox.lock().unwrap().refresh);
}
#[test]
fn utc_timestamp_handles_epoch_and_leap_day() {
    assert_eq!(ui::timestamp(0), "1970-01-01 00:00:00.000");
    assert_eq!(ui::timestamp(1709164800123), "2024-02-29 00:00:00.123");
}

#[test]
fn completed_file_job_repaints_even_when_connection_error_is_unchanged() {
    let mut s = State {
        connection_error: Some("offline".into()),
        ..Default::default()
    };
    s.mailbox.lock().unwrap().update = Some(Err("offline".into()));
    let job = Arc::new(session::Job::default());
    *job.result.lock().unwrap() = Some(Ok(session::ResultValue::Cancelled));
    s.workspace.job = Some(job);
    assert!(poll(&mut s, 0.0, &mut Tx::default()));
    assert!(s.workspace.notice.is_some());
}

#[test]
fn diagram_and_detail_expand_buttons_share_one_state() {
    let mut rt = runtime((1440.0, 900.0));
    tap(&mut rt, "tab-3");
    tap(&mut rt, "expand-detail");
    assert!(rt.state.workspace.detail_expanded);
    tap(&mut rt, "diagram-expand");
    assert!(!rt.state.workspace.detail_expanded);
    tap(&mut rt, "diagram-expand");
    assert!(rt.state.workspace.detail_expanded);
    tap(&mut rt, "tab-1");
    tap(&mut rt, "expand-detail");
    assert!(!rt.state.workspace.detail_expanded);
}
