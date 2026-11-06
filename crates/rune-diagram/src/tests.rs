use super::*;
use rune::{Cx, El, Env, Runtime, WheelEvent, col};
use rune_kit::Kit;
use serde_json::json;

#[test]
fn zoom_keeps_anchor_stable_and_fit_handles_invalid_sizes() {
    let mut c = Camera {
        x: 30.0,
        y: -40.0,
        scale: 0.8,
    };
    let anchor = [230.0, 160.0];
    let world = [(anchor[0] - c.x) / c.scale, (anchor[1] - c.y) / c.scale];
    c.zoom(2.0, anchor, anchor);
    assert!((world[0] * c.scale + c.x - anchor[0]).abs() < 0.01);
    assert!((world[1] * c.scale + c.y - anchor[1]).abs() < 0.01);
    c.zoom(1e10, anchor, anchor);
    assert_eq!(c.scale, 4.0);
    let before = c;
    c.zoom(f32::NAN, anchor, anchor);
    assert_eq!(c, before);
    assert_eq!(Camera::fit([0.0, 0.0], [100.0, 100.0]), Camera::default());
    assert_eq!(Camera::fit([1048.0, 548.0], [1000.0, 500.0]).scale, 1.0);
}

#[test]
fn graph_validation_rejects_ambiguous_ids_dangling_edges_and_bad_geometry() {
    let mut g = fixture();
    assert!(g.validate().is_ok());
    g.nodes.push(g.nodes[0].clone());
    assert!(g.validate().is_err());
    g.nodes.pop();
    g.edges.push(Edge::new("a", "missing"));
    assert!(g.validate().is_err());
    g.edges.pop();
    g.nodes[0].rect[0] = f32::NAN;
    assert!(g.validate().is_err());
}

fn fixture() -> Graph {
    Graph {
        nodes: vec![
            Node::new(
                "a",
                "Input",
                [0.0, 0.0, 240.0, 260.0],
                NodeBody::Json(Some(json!({"nested":{"value":42},"null":null}))),
            ),
            Node::new(
                "b",
                "Result",
                [350.0, 0.0, 240.0, 260.0],
                NodeBody::Text("Captured result".into()),
            ),
        ],
        edges: vec![Edge::new("a", "b").active(true)],
    }
}

#[derive(Default)]
struct State {
    diagram: DiagramState,
}
fn view(s: &State, _: &mut Cx<State>) -> El {
    col().w_full().h_full().child(
        Diagram::new("test", &fixture(), &s.diagram, |s: &mut State| {
            &mut s.diagram
        })
        .build(&Kit::neutral(true)),
    )
}
fn tap(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.1);
}

#[test]
fn runtime_camera_selection_disclosure_and_wheel_work_together() {
    let mut rt = Runtime::new(
        State::default(),
        view,
        (900.0, 600.0),
        Env {
            system_dark: true,
            scale: 1.0,
        },
    );
    rt.frame(0.0);
    rt.frame(0.02);
    assert!(rt.state.diagram.fitted);
    let scale = rt.state.diagram.camera.scale;
    tap(&mut rt, "test:zoom-in");
    assert!(rt.state.diagram.camera.scale > scale);
    tap(&mut rt, "test:actual-size");
    assert!((rt.state.diagram.camera.scale - 1.0).abs() < 0.001);
    tap(&mut rt, "test:fit");
    tap(&mut rt, "test:node:1:a:select");
    assert_eq!(rt.state.diagram.selected.as_deref(), Some("a"));
    tap(&mut rt, "test:node:1:a/nested:toggle");
    assert!(rt.state.diagram.content.expanded("test:node:1:a/nested"));
    let (x, y) = rt.find("test:node:1:a:content").unwrap();
    let before = rt.state.diagram.camera;
    rt.wheel_event(WheelEvent::new(0.0, -60.0).at(x, y));
    rt.frame(0.15);
    assert_eq!(
        rt.state.diagram.camera, before,
        "card scrolling must not zoom the diagram"
    );
    let (x, y) = rt.find("test:viewport").unwrap();
    let before = rt.state.diagram.camera;
    rt.wheel_event(WheelEvent::new(0.0, -60.0).at(x, y));
    rt.frame(0.2);
    assert!(rt.state.diagram.camera.scale > before.scale);
    tap(&mut rt, "test:fit");
    let before = rt.state.diagram.camera;
    let (x, y) = rt.find("test:viewport").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_move(x + 50.0, y + 20.0);
    rt.pointer_up(x + 50.0, y + 20.0);
    rt.frame(0.3);
    assert!((rt.state.diagram.camera.x - before.x - 50.0).abs() < 0.1);
    assert!((rt.state.diagram.camera.y - before.y - 20.0).abs() < 0.1);
    rt.key(rune::KeyEvent {
        key: rune::Key::Text("0".into()),
        mods: rune::Mods::default(),
    });
    rt.frame(0.4);
    assert_eq!(rt.state.diagram.camera, before);
}

#[test]
fn disclosure_state_is_bounded() {
    let mut s = ContentState::default();
    for i in 0..1000 {
        s.toggle(i.to_string());
        s.more(i.to_string());
    }
    assert!(!s.expanded("999"));
    assert_eq!(s.visible("999"), 12);
}

#[test]
fn first_fit_contains_scaled_cards_without_a_startup_spring() {
    let mut rt = Runtime::new(
        State::default(),
        view,
        (400.0, 300.0),
        Env {
            system_dark: true,
            scale: 1.0,
        },
    );
    rt.frame(0.0);
    rt.frame(0.02);
    rt.frame(0.04);
    let (v, _) = rt.presented("test:viewport").unwrap();
    for key in ["test:node:1:a", "test:node:1:b"] {
        let (r, _) = rt.presented(key).unwrap();
        assert!(
            r[0] >= v[0]
                && r[1] >= v[1]
                && r[0] + r[2] <= v[0] + v[2] + 0.1
                && r[1] + r[3] <= v[1] + v[3] + 0.1,
            "{r:?} outside {v:?}"
        );
    }
}

#[test]
fn automatic_fit_tracks_resizing_but_manual_camera_is_preserved() {
    let mut rt = Runtime::new(
        State::default(),
        view,
        (400.0, 300.0),
        Env {
            system_dark: true,
            scale: 1.0,
        },
    );
    rt.frame(0.0);
    rt.frame(0.1);
    for size in [(700.0, 500.0), (360.0, 260.0)] {
        rt.set_viewport(size, 1.0);
        rt.frame(0.1);
        rt.frame(0.1);
        assert_eq!(
            rt.state.diagram.camera,
            Camera::fit(rt.state.diagram.viewport, fixture().size())
        );
    }
    tap(&mut rt, "test:zoom-in");
    let manual = rt.state.diagram.camera;
    rt.set_viewport((650.0, 450.0), 1.0);
    rt.frame(0.1);
    rt.frame(0.1);
    assert_eq!(rt.state.diagram.camera, manual);
    tap(&mut rt, "test:fit");
    assert_eq!(
        rt.state.diagram.camera,
        Camera::fit(rt.state.diagram.viewport, fixture().size())
    );
}

#[test]
fn explicit_fit_request_overrides_readable_initial_focus() {
    let mut state = DiagramState::default();
    state.viewport = [400.0, 300.0];
    let size = [1400.0, 600.0];
    let focus = Some([400.0, 100.0, 320.0, 340.0]);
    state.initialize(size, focus);
    assert_eq!(state.camera.scale, 1.0);
    state.request_fit();
    state.initialize(size, focus);
    assert_eq!(state.camera, Camera::fit(state.viewport, size));
    assert!(!state.initial_view);
}
