use super::*;

fn runtime(fixture: &str) -> rune::Runtime<State> {
    let mut s = State::default();
    demo::scenario(&mut s, fixture).unwrap();
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
    rt
}

#[test]
fn loading_tabs_keep_context_and_remove_placeholders_when_details_arrive() {
    let mut rt = runtime("loading");
    rt.set_system_reduced_motion(true);
    for tab in 0..=4 {
        rt.update(move |s, _| s.tab = tab);
        rt.frame(0.1 + f64::from(tab));
        assert!(rt.find("detail-loading").is_some());
        assert!(rt.find("copy-id").is_some());
        let tree = rt.accessibility_json().to_string();
        assert!(tree.contains(detail_states::tab_name(tab)));
        assert!(tree.contains("Loading request details"));
        assert!(
            !tree.contains("loading-preview"),
            "decorative skeleton stays out of the accessibility tree"
        );
        let elements = rt.elements();
        let loops: Vec<_> = elements.iter().filter(|e| e.loop_phase.is_some()).collect();
        assert!(!loops.is_empty(), "fixture includes the loading indicator");
        assert!(
            loops.iter().all(|e| !e.moving && e.loop_phase == Some(0.0)),
            "reduced motion stops the loading animation"
        );
    }
    rt.update(|s, _| s.set_detail(demo::exchange(12)));
    rt.frame(6.0);
    assert!(rt.find("detail-loading").is_none());
    assert!(rt.find("set-baseline").is_some());
    assert_eq!(rt.state.selected, Some(12));
    assert!(
        rt.elements().iter().all(|e| e.loop_phase.is_none()),
        "loading animation is removed with the skeleton"
    );
}

#[test]
fn detail_failure_retry_requests_a_refresh_and_connection_loss_stops_loading() {
    let mut rt = runtime("detail-failed");
    assert!(rt.find("detail-loading").is_none());
    let (x, y) = rt.find("retry-detail").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.1);
    assert!(rt.state.detail_error.is_none());
    assert!(rt.state.mailbox.lock().unwrap().refresh);
    assert!(rt.find("detail-loading").is_some());
    rt.state.mailbox.lock().unwrap().update = Some(Err("Relay unavailable".into()));
    rt.update(|s, tx| {
        poll(s, 0.2, tx);
    });
    rt.frame(0.2);
    assert!(rt.find("detail-loading").is_none());
    assert!(rt.find("detail-unavailable").is_some());
    assert!(
        rt.accessibility_json()
            .to_string()
            .contains("Reconnecting automatically")
    );
}

#[test]
fn pending_empty_and_saved_missing_bodies_have_distinct_states() {
    let mut rt = runtime("pending-response");
    assert!(rt.find("response-pending").is_some());
    rt.update(|s, _| s.set_detail(demo::exchange(12)));
    rt.frame(0.1);
    assert!(rt.find("response-pending").is_none());
    assert!(rt.find("response-json:scroll").is_some());
    let empty = runtime("empty-response");
    assert!(empty.find("body-empty").is_some());
    let saved = runtime("summary-only");
    assert!(saved.find("detail-unavailable").is_some());
    assert!(saved.find("retry-detail").is_none());
    assert!(saved.find("detail-loading").is_none());
}

#[test]
fn late_detail_response_does_not_replace_the_new_selection() {
    let mut rt = runtime("loading");
    rt.update(|s, _| s.select(11));
    let snapshot = rt.state.snapshot.clone();
    rt.state.mailbox.lock().unwrap().update = Some(Ok(feed::Update {
        snapshot,
        detail: Some((12, Ok(demo::exchange(12)))),
    }));
    rt.update(|s, tx| {
        poll(s, 0.1, tx);
    });
    assert_eq!(rt.state.selected, Some(11));
    assert_eq!(rt.state.detail.as_ref().unwrap().summary.id, 11);
}
