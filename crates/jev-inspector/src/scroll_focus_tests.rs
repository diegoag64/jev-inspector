use super::showcase_tests::{click, press, runtime};
use super::*;

const PROPERTIES: &str = "request-diagram:node:5:state:content";
const THUMB: &str = "request-diagram:node:5:state:scroll-thumb";
const TRACK: &str = "request-diagram:node:5:state:scrollbar";

fn settle(rt: &mut rune::Runtime<State>) {
    for _ in 0..4 {
        rt.frame(0.2);
    }
}

fn rect(rt: &rune::Runtime<State>, key: &str) -> [f32; 4] {
    rt.presented(key)
        .map(|v| v.0)
        .or_else(|| {
            rt.elements()
                .into_iter()
                .find(|e| e.path.ends_with(key))
                .map(|e| e.rect)
        })
        .unwrap_or_else(|| panic!("Missing {key}"))
}

fn assert_properties_focus(rt: &rune::Runtime<State>) {
    let focused = rt.elements().into_iter().find(|e| e.focused).unwrap();
    assert_eq!(focused.key.as_deref(), Some(PROPERTIES));
    let r = focused.rect;
    let v = rect(rt, "request-diagram:viewport");
    assert!(
        r[0] >= v[0]
            && r[1] >= v[1]
            && r[0] + r[2] <= v[0] + v[2] + 0.5
            && r[1] + r[3] <= v[1] + v[3] + 0.5,
        "focus {r:?} in {v:?}"
    );
}

fn drag(rt: &mut rune::Runtime<State>, distance: f32) {
    let (x, y) = rt.find(THUMB).unwrap();
    rt.pointer_down(x, y);
    rt.frame(0.02);
    // Include intermediate frames so rebuilt callbacks retain the drag origin.
    for i in 1..=5 {
        rt.pointer_move(x, y + distance * i as f32 / 5.0);
        rt.frame(0.04);
    }
    rt.pointer_up(x, y + distance);
    settle(rt);
}

#[test]
fn property_drag_matches_visible_travel_and_keeps_keyboard_in_its_pane() {
    for renderer_scale in [1.0, 1.5] {
        for zoom in [0.8, 1.0, 1.2] {
            let mut rt = runtime();
            rt.set_viewport((1100.0, 700.0), renderer_scale);
            rt.update(|s, _| showcase::select_example(s, "support-routing").unwrap());
            settle(&mut rt);
            click(&mut rt, "request-diagram:node:6:answer:select");
            rt.update(|s, _| s.diagram.zoom(zoom / s.diagram.camera.scale));
            press(&mut rt, rune::Key::Home, false);
            press(&mut rt, rune::Key::Down, false);
            settle(&mut rt);
            let traffic = rect(&rt, "requests/viewport/rows");
            let thumb = rect(&rt, THUMB);
            let track = rect(&rt, TRACK);
            let travel = track[3] - thumb[3] - 2.0 * (thumb[1] - track[1]);
            assert!(travel > 2.0);
            drag(&mut rt, travel * 0.5);
            let moved = rect(&rt, THUMB)[1] - thumb[1];
            assert!(
                (moved - travel * 0.5).abs() < 0.6,
                "zoom={zoom}, renderer={renderer_scale}: moved {moved}, expected {}",
                travel * 0.5
            );
            assert_properties_focus(&rt);
            press(&mut rt, rune::Key::Home, false);
            settle(&mut rt);
            drag(&mut rt, travel);
            let message = rt
                .elements()
                .into_iter()
                .find(|e| e.text.as_deref() == Some("My invoice was charged twice."))
                .unwrap()
                .rect;
            let v = rect(&rt, PROPERTIES);
            assert!(
                message[1] + message[3] <= v[1] + v[3] + 0.6,
                "final scalar clipped: {message:?} in {v:?}"
            );
            for key in [
                rune::Key::PageUp,
                rune::Key::PageDown,
                rune::Key::Home,
                rune::Key::End,
            ] {
                press(&mut rt, key, false);
                settle(&mut rt);
                assert_properties_focus(&rt);
                assert_eq!(rect(&rt, "requests/viewport/rows"), traffic);
                assert!((rt.state.diagram.camera.scale - zoom).abs() < 0.001);
            }
            // The track, not just its thumb, must retain ownership of keys.
            press(&mut rt, rune::Key::Home, false);
            settle(&mut rt);
            let track = rect(&rt, TRACK);
            rt.pointer_down(track[0] + track[2] / 2.0, track[1] + track[3] - 1.0);
            rt.pointer_up(track[0] + track[2] / 2.0, track[1] + track[3] - 1.0);
            settle(&mut rt);
            press(&mut rt, rune::Key::Home, false);
            settle(&mut rt);
            assert_properties_focus(&rt);
            assert_eq!(rect(&rt, "requests/viewport/rows"), traffic);
            assert!((rect(&rt, THUMB)[1] - thumb[1]).abs() < 0.6);
        }
    }
}

#[test]
fn paging_nested_disclosure_moves_focus_to_visible_properties() {
    let mut rt = runtime();
    rt.update(|s, _| {
        showcase::select_example(s, "support-routing").unwrap();
        let mut detail = s.detail.clone().unwrap();
        let nested: serde_json::Map<String, serde_json::Value> = (0..24)
            .map(|i| (format!("property{i:02}"), serde_json::json!(i)))
            .collect();
        detail.request.json.as_mut().unwrap()["state"] = serde_json::json!({"nested":nested});
        s.set_detail(detail);
    });
    settle(&mut rt);
    click(&mut rt, "request-diagram:node:6:answer:select");
    press(&mut rt, rune::Key::Home, false);
    press(&mut rt, rune::Key::Down, false);
    assert!(rt.elements().iter().any(|e| {
        e.focused
            && e.key
                .as_deref()
                .is_some_and(|k| k.ends_with("/nested:toggle"))
    }));
    press(&mut rt, rune::Key::Text(" ".into()), false);
    settle(&mut rt);
    let zoom = rt.state.diagram.camera.scale;
    press(&mut rt, rune::Key::End, false);
    settle(&mut rt);
    assert_properties_focus(&rt);
    assert!(
        rt.state
            .diagram
            .content
            .expanded("request-diagram:node:5:state/nested")
    );
    assert_eq!(rt.state.diagram.selected.as_deref(), Some("state"));
    assert_eq!(rt.state.diagram.camera.scale, zoom);
    press(&mut rt, rune::Key::Home, false);
    settle(&mut rt);
    press(&mut rt, rune::Key::Down, false);
    assert!(rt.elements().iter().any(|e| {
        e.focused
            && e.key
                .as_deref()
                .is_some_and(|k| k.ends_with("/nested:toggle"))
    }));
    press(&mut rt, rune::Key::PageDown, false);
    settle(&mut rt);
    assert_properties_focus(&rt);
}

#[test]
fn requested_control_focus_respects_modal_scope_and_last_request() {
    let mut rt = runtime();
    rt.update(|_, tx| tx.focus_control("help"));
    settle(&mut rt);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.focused && e.key.as_deref() == Some("help"))
    );
    rt.update(|_, tx| tx.focus_control("does-not-exist"));
    settle(&mut rt);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.focused && e.key.as_deref() == Some("help"))
    );
    rt.update(|_, tx| {
        tx.focus_control("help");
        tx.blur();
    });
    settle(&mut rt);
    assert!(!rt.elements().iter().any(|e| e.focused));
    rt.update(|_, tx| {
        tx.blur();
        tx.focus_control("help");
    });
    settle(&mut rt);
    assert!(
        rt.elements()
            .iter()
            .any(|e| e.focused && e.key.as_deref() == Some("help"))
    );
    click(&mut rt, "help");
    let before = rt.elements().into_iter().find(|e| e.focused).unwrap().path;
    rt.update(|_, tx| tx.focus_control("request-filter"));
    settle(&mut rt);
    assert_eq!(
        rt.elements().into_iter().find(|e| e.focused).unwrap().path,
        before
    );
}

#[test]
fn inactive_properties_drag_and_track_activate_owner_before_keyboard_input() {
    for renderer_scale in [1.0, 1.5] {
        for track_click in [false, true] {
            for select_other in [false, true] {
                let mut rt = runtime();
                rt.set_viewport((1100.0, 700.0), renderer_scale);
                rt.update(|s, _| showcase::select_example(s, "support-routing").unwrap());
                settle(&mut rt);
                if select_other {
                    click(&mut rt, "request-diagram:node:6:answer:select");
                }
                click(&mut rt, "request-diagram:fit");
                settle(&mut rt);
                assert!(
                    !rt.elements()
                        .iter()
                        .find(|e| e.key.as_deref() == Some(PROPERTIES))
                        .unwrap()
                        .focusable
                );
                let camera = rt.state.diagram.camera;
                let traffic = rect(&rt, "requests/viewport/rows");
                let top = rect(&rt, THUMB)[1];
                if track_click {
                    let r = rect(&rt, TRACK);
                    let (x, y) = (r[0] + r[2] / 2.0, r[1] + r[3] - 0.5);
                    // Down, up and the key can arrive before a view has rebuilt.
                    rt.pointer_down(x, y);
                    rt.pointer_up(x, y);
                    rt.key(rune::KeyEvent {
                        key: rune::Key::Home,
                        mods: Default::default(),
                    });
                    rt.frame(0.3);
                    settle(&mut rt);
                } else {
                    drag(&mut rt, 15.0);
                    assert!(rect(&rt, THUMB)[1] > top + 1.0);
                    press(&mut rt, rune::Key::Home, false);
                    settle(&mut rt);
                }
                assert_eq!(rt.state.diagram.selected.as_deref(), Some("state"));
                assert_properties_focus(&rt);
                assert_eq!(
                    rt.state.diagram.camera, camera,
                    "pointer activation must not reframe underneath a drag"
                );
                assert_eq!(rect(&rt, "requests/viewport/rows"), traffic);
                assert!((rect(&rt, THUMB)[1] - top).abs() < 0.6);
                for key in [
                    rune::Key::End,
                    rune::Key::PageUp,
                    rune::Key::PageDown,
                    rune::Key::Home,
                ] {
                    press(&mut rt, key, false);
                    settle(&mut rt);
                    assert_properties_focus(&rt);
                    assert_eq!(rect(&rt, "requests/viewport/rows"), traffic);
                }
                // A different card's activation removes properties from traversal again.
                click(&mut rt, "request-diagram:node:6:answer:select");
                assert!(
                    !rt.elements()
                        .iter()
                        .find(|e| e.key.as_deref() == Some(PROPERTIES))
                        .unwrap()
                        .focusable
                );
            }
        }
    }
}
