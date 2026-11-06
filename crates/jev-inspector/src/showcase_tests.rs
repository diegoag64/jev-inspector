use super::*;

fn runtime() -> rune::Runtime<State> {
    let mut s = State::default();
    showcase::populate(&mut s);
    let mut rt = rune::Runtime::new(
        s,
        view,
        (1100.0, 700.0),
        rune::Env {
            system_dark: true,
            scale: 1.0,
        },
    );
    rt.on_key = Some(workspace::on_key);
    rt.frame(0.0);
    rt.frame(1.0);
    rt
}

fn wait_job(rt: &mut rune::Runtime<State>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while rt.state.workspace.job.is_some() {
        assert!(std::time::Instant::now() < deadline, "file job timed out");
        rt.state.poll_job();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    rt.frame(0.02);
}

#[test]
fn showcase_session_round_trip_preserves_every_capture_and_filtered_exports() {
    let mut rt = runtime();
    rt.state.workspace.starred.insert(4);
    let originals = serde_json::to_value(&rt.state.workspace.cache).unwrap();
    let path = std::env::temp_dir().join(format!(
        "jev-showcase-roundtrip-{}.json",
        std::process::id()
    ));
    rt.update(|s, tx| session::save_dialog(s, tx, 0));
    rt.frame(0.01);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(0.01);
    wait_job(&mut rt);
    let saved = session::read(&path).unwrap();
    assert_eq!(serde_json::to_value(&saved.exchanges).unwrap(), originals);
    assert_eq!(saved.snapshot.requests.len(), 14);
    assert!(saved.starred.contains(&4));
    rt.update(session::open_dialog);
    rt.frame(0.01);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(0.01);
    wait_job(&mut rt);
    assert_eq!(
        serde_json::to_value(&rt.state.workspace.cache).unwrap(),
        originals
    );
    assert!(rt.state.workspace.starred.contains(&4));
    rt.update(|s, _| {
        s.return_live();
        s.filter = TextEdit::new("status:200");
        s.filter_rows();
    });
    let expected: Vec<_> = rt.state.visible.iter().map(|r| r.id).collect();
    rt.update(|s, tx| session::save_dialog(s, tx, 0));
    rt.frame(0.01);
    rt.resolve_file_dialog(Some(path.clone())).unwrap();
    rt.frame(0.01);
    wait_job(&mut rt);
    let filtered = session::read(&path).unwrap();
    assert_eq!(filtered.exchanges.len(), expected.len());
    for id in expected {
        assert_eq!(
            serde_json::to_value(&filtered.exchanges[&id]).unwrap(),
            originals[id.to_string()]
        );
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn pending_open_and_save_pickers_pause_tour_before_cancel_or_failed_import() {
    let mut rt = runtime();
    for save in [false, true] {
        rt.update(|s, tx| {
            s.showcase.as_mut().unwrap().playing = true;
            showcase::tick(s, 0.0, tx);
            if save {
                session::save_dialog(s, tx, 0);
            } else {
                session::open_dialog(s, tx);
            }
        });
        rt.frame(0.01);
        let selected = rt.state.selected;
        assert!(rt.file_dialog().is_some());
        rt.update(|s, tx| {
            showcase::tick(s, 10.0, tx);
        });
        assert_eq!(rt.state.selected, selected);
        assert!(!rt.state.showcase.as_ref().unwrap().playing);
        rt.resolve_file_dialog(None).unwrap();
        rt.frame(0.01);
        assert_eq!(rt.state.selected, selected);
    }
    rt.update(|s, tx| {
        s.showcase.as_mut().unwrap().playing = true;
        session::open_dialog(s, tx);
    });
    rt.frame(0.01);
    rt.resolve_file_dialog(Some(
        std::env::temp_dir().join("jev-missing-showcase-folder/no-file.json"),
    ))
    .unwrap();
    rt.frame(0.01);
    wait_job(&mut rt);
    assert!(!rt.state.showcase.as_ref().unwrap().playing);
    assert_eq!(rt.state.selected, Some(14));
    assert!(rt.state.workspace.notice.as_ref().unwrap().1);
}

#[test]
fn catalog_exercises_typed_answers_payloads_and_protocol_edge_cases() {
    let mut s = State::default();
    showcase::populate(&mut s);
    assert_eq!(s.snapshot.requests.len(), 14);
    assert_eq!(s.workspace.cache.len(), 14);
    for example in [
        "support-routing",
        "fraud-check",
        "incident-score",
        "invoice-box",
        "return-review",
        "many-choices",
        "inventory",
        "unicode",
        "models",
        "rate-limit",
        "auth-error",
        "empty",
        "non-json",
        "slow-analysis",
    ] {
        showcase::select_example(&mut s, example).unwrap();
        let d = s.detail.as_ref().unwrap();
        assert_eq!(Some(d.summary.id), s.selected);
        assert_eq!(
            d.summary.request_id.as_deref(),
            Some(format!("demo-{example}").as_str())
        );
        assert!(d.response.complete);
        if example == "empty" {
            assert_eq!(d.response.wire_bytes, 0);
            assert!(d.response.json.is_none());
        }
        if example == "non-json" {
            assert!(d.response.wire_bytes > 0);
            assert!(d.response.json.is_none());
            assert!(!d.response.notes.is_empty());
        }
        if example == "inventory" {
            assert!(s.request_text.tree);
            assert!(d.request.wire_bytes > 20_000);
        }
        if example == "slow-analysis" {
            assert!(d.summary.duration_ms.unwrap() > 2000.0);
        }
    }
}

#[test]
fn tour_cycles_layouts_and_yields_to_manual_selection_and_dialogs() {
    let mut rt = runtime();
    rt.update(|s, tx| {
        s.showcase.as_mut().unwrap().playing = true;
        assert!(showcase::tick(s, 1.0, tx));
    });
    rt.update(|s, tx| {
        assert!(!showcase::tick(s, 4.0, tx));
        assert!(showcase::tick(s, 4.3, tx));
    });
    assert_eq!(rt.state.selected, Some(13));
    assert_eq!(rt.state.tab, 2);
    rt.update(|s, _| s.select(4));
    assert!(!rt.state.showcase.as_ref().unwrap().playing);
    rt.update(|s, tx| {
        s.showcase.as_mut().unwrap().playing = true;
        s.workspace.help = true;
        assert!(showcase::tick(s, 8.0, tx));
    });
    assert_eq!(rt.state.selected, Some(4));
    assert!(!rt.state.showcase.as_ref().unwrap().playing);
    rt.update(|s, _| {
        s.workspace.help = false;
        s.return_live();
    });
    assert_eq!(rt.state.workspace.cache.len(), 14);
}

#[test]
fn mixed_layout_transition_settles_and_rapid_selection_keeps_only_current_content() {
    let mut rt = runtime();
    for slug in [
        "inventory",
        "invoice-box",
        "empty",
        "rate-limit",
        "unicode",
        "return-review",
    ]
    .iter()
    {
        rt.update(|s, tx| {
            showcase::select_example(s, slug).unwrap();
            tx.animated();
        });
        rt.frame(0.01);
        rt.frame(0.015);
        let elements = rt.elements();
        let surfaces: Vec<_> = elements
            .iter()
            .filter(|e| {
                e.key
                    .as_deref()
                    .is_some_and(|k| k.starts_with("detail-content-"))
            })
            .collect();
        assert_eq!(
            surfaces.len(),
            1,
            "old detail surface must disappear immediately, even during rapid switching"
        );
        assert!(surfaces[0].moving, "new detail surface should animate");
        assert!(surfaces[0].opacity < 1.0, "transition actually fades in");
        assert!(
            elements
                .iter()
                .all(|e| e.rect.iter().all(|v| v.is_finite()))
        );
        assert_eq!(
            rt.state.detail.as_ref().unwrap().summary.id,
            rt.state.selected.unwrap()
        );
    }
    rt.frame(3.0);
    let elements = rt.elements();
    let body = elements
        .iter()
        .find(|e| {
            e.key
                .as_deref()
                .is_some_and(|k| k.starts_with("detail-content-"))
        })
        .unwrap();
    assert!(!body.moving);
    assert!((body.opacity - 1.0).abs() < 0.001);
    assert!(
        elements.iter().all(|e| e.loop_phase.is_none()),
        "idle examples have no endless animation"
    );
}

#[test]
fn reduced_motion_removes_detail_travel_and_tour_restart_restores_filtered_examples() {
    let mut rt = runtime();
    rt.set_system_reduced_motion(true);
    rt.update(|s, tx| {
        showcase::select_example(s, "invoice-box").unwrap();
        tx.animated();
    });
    rt.frame(0.01);
    rt.frame(0.04);
    for e in rt.elements().iter().filter(|e| {
        e.key
            .as_deref()
            .is_some_and(|k| k.starts_with("detail-content-"))
    }) {
        assert!(
            (e.rect[1] - e.target[1]).abs() < 0.1,
            "reduced motion has no lift"
        );
    }
    rt.update(|s, _| s.clear_view());
    rt.frame(2.0);
    let (x, y) = rt.find("showcase-tour").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(2.1);
    assert_eq!(rt.state.visible.len(), 14);
    assert!(rt.state.showcase.as_ref().unwrap().playing);
}

#[test]
fn expanded_inventory_scrollbar_moves_content_without_changing_the_exchange() {
    let mut rt = runtime();
    rt.update(|s, _| {
        showcase::select_example(s, "inventory").unwrap();
        s.request_text.outline.expand_all();
    });
    rt.frame(0.4);
    rt.frame(0.1);
    let (x, y) = rt
        .find("json-tree-scroll-thumb")
        .expect("overflowing tree has a scrollbar");
    rt.pointer_down(x, y);
    rt.pointer_move(x, y + 80.0);
    rt.pointer_up(x, y + 80.0);
    rt.frame(0.3);
    rt.frame(0.1);
    let (_, after) = rt.find("json-tree-scroll-thumb").unwrap();
    assert!(after > y + 20.0, "tree thumb follows its viewport");
    assert_eq!(rt.state.selected, Some(7));
    rt.update(|s, _| s.request_text.outline.collapse_all());
    rt.frame(0.4);
    rt.frame(0.1);
    assert!(
        rt.find("json-tree-scroll-thumb").is_none(),
        "collapsed content no longer overflows"
    );
}

#[test]
fn compact_diagrams_open_on_readable_answer_and_fit_uses_the_settled_viewport() {
    let mut rt = runtime();
    for slug in [
        "support-routing",
        "fraud-check",
        "invoice-box",
        "many-choices",
    ] {
        rt.update(|s, _| {
            showcase::select_example(s, slug).unwrap();
        });
        rt.frame(0.1);
        rt.frame(0.3);
        assert_eq!(
            rt.state.diagram.camera.scale, 1.0,
            "readable initial scale for {slug}"
        );
        let (v, _) = rt.presented("request-diagram:viewport").unwrap();
        let (a, _) = rt.presented("request-diagram:node:6:answer").unwrap();
        assert!(a[0] >= v[0] && a[0] + a[2] <= v[0] + v[2] + 0.1);
        assert!(a[1] >= v[1] && a[1] < v[1] + 40.0);
        let (x, y) = rt.find("request-diagram:fit").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.3);
        let first_fit = rt.state.diagram.camera;
        rt.frame(0.2);
        let (x, y) = rt.find("request-diagram:fit").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.3);
        assert_eq!(
            rt.state.diagram.camera, first_fit,
            "Fit is stable after final layout"
        );
    }
}

#[test]
fn json_scrollbar_axes_preserve_each_other_and_short_content_hides_them() {
    let mut s = State::default();
    demo::scenario(&mut s, "long-json").unwrap();
    s.tab = 2;
    let mut rt = rune::Runtime::new(
        s,
        view,
        (1100.0, 700.0),
        rune::Env {
            system_dark: true,
            scale: 1.0,
        },
    );
    rt.frame(0.0);
    rt.frame(0.3);
    let (x, y) = rt
        .find("detail-horizontal-thumb")
        .expect("long scalar needs horizontal thumb");
    rt.pointer_down(x, y);
    rt.pointer_move(x + 80.0, y);
    rt.pointer_up(x + 80.0, y);
    rt.frame(0.2);
    rt.frame(0.1);
    let before = rt.presented("response-json:content").unwrap().0;
    let (x, y) = rt
        .find("detail-vertical-thumb")
        .expect("long JSON needs vertical thumb");
    rt.pointer_down(x, y);
    rt.pointer_move(x, y + 50.0);
    rt.pointer_up(x, y + 50.0);
    rt.frame(0.2);
    rt.frame(0.1);
    let after = rt.presented("response-json:content").unwrap().0;
    assert!(
        (after[0] - before[0]).abs() < 0.1,
        "vertical scroll preserves horizontal offset"
    );
    assert!(after[1] < before[1] - 10.0);
    rt.update(|s, _| {
        showcase::populate(s);
        showcase::select_example(s, "empty").unwrap();
    });
    rt.frame(0.3);
    rt.frame(0.1);
    assert!(rt.find("detail-horizontal-thumb").is_none());
    assert!(rt.find("detail-vertical-thumb").is_none());
}

#[test]
fn find_focus_and_copy_follow_current_capture_during_interrupted_transitions() {
    fn focused_within(node: &serde_json::Value, key: &str, inside: bool) -> bool {
        let inside = inside || node["key"] == key;
        (inside
            && node["states"]
                .as_array()
                .is_some_and(|states| states.contains(&serde_json::json!("focused"))))
            || node["children"].as_array().is_some_and(|children| {
                children
                    .iter()
                    .any(|child| focused_within(child, key, inside))
            })
    }
    for reduced in [false, true] {
        let mut rt = runtime();
        rt.set_system_reduced_motion(reduced);
        for tab in [1, 2] {
            rt.update(|s, _| {
                s.select(8);
                s.tab = tab;
                s.request_text.tree = false;
                s.response_text.tree = false;
            });
            rt.frame(0.3);
            for id in [7, 6, 5] {
                rt.key(rune::KeyEvent {
                    key: rune::Key::Command('f'),
                    mods: rune::Mods {
                        cmd: true,
                        ..Default::default()
                    },
                });
                rt.frame(0.01);
                rt.key(rune::KeyEvent {
                    key: rune::Key::Down,
                    mods: rune::Mods {
                        alt: true,
                        ..Default::default()
                    },
                });
                rt.frame(0.01);
                rt.frame(0.015);
                assert_eq!(rt.state.selected, Some(id));
                let search = if tab == 1 {
                    "request-json:search"
                } else {
                    "response-json:search"
                };
                let focused: Vec<_> = rt.elements().into_iter().filter(|e| e.focused).collect();
                assert_eq!(
                    focused.len(),
                    1,
                    "one focused control after keyboard navigation: tab={tab}, id={id}, reduced={reduced}"
                );
                assert!(
                    focused_within(&rt.accessibility_json(), search, false),
                    "current Find field owns focus"
                );
                assert!(!focused[0].exiting && !focused[0].inert);
                rt.key(rune::KeyEvent {
                    key: rune::Key::Text("a".into()),
                    mods: Default::default(),
                });
                rt.frame(0.01);
                let doc = if tab == 1 {
                    &rt.state.request_text
                } else {
                    &rt.state.response_text
                };
                assert!(
                    doc.search.text.ends_with('a'),
                    "typing reaches the current Find field"
                );
                let copy = if tab == 1 {
                    "request-json:copy"
                } else {
                    "response-json:copy"
                };
                let (x, y) = rt.find(copy).unwrap();
                rt.pointer_down(x, y);
                rt.pointer_up(x, y);
                rt.frame(0.01);
                let copied: serde_json::Value =
                    serde_json::from_str(rt.copy_request.take().unwrap().content().text()).unwrap();
                let capture = if tab == 1 {
                    &rt.state.detail.as_ref().unwrap().request
                } else {
                    &rt.state.detail.as_ref().unwrap().response
                };
                assert_eq!(Some(copied), capture.json);
            }
        }
    }
}
