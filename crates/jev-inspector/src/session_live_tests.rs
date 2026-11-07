use super::*;
use std::{net::TcpListener, time::Instant};

fn exercise(restart: bool, unavailable: bool) {
    let mut state = State::default();
    crate::demo::populate(&mut state);
    state.errors_only = true;
    state.filter_rows();
    state.demo = false;
    if unavailable {
        state.workspace.cache.insert(7, crate::demo::exchange(7));
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = stop.clone();
    let snapshot = state.snapshot.clone();
    let server = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(8);
        let mut snapshots = 0;
        while !server_stop.load(Ordering::Relaxed) && Instant::now() < until {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(2));
                continue;
            };
            // Windows accepted sockets inherit the listener's nonblocking mode.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 2048];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).unwrap();
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..n]);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(
                request
                    .to_lowercase()
                    .contains("authorization: bearer synthetic-test-token")
            );
            let route = request
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            let (status, body) = match route {
                "/api/requests" => {
                    snapshots += 1;
                    let mut current = snapshot.clone();
                    if restart && snapshots > 1 {
                        current.instance_id = "new-relay".into();
                    }
                    if unavailable && snapshots > 1 {
                        (503, b"{}".to_vec())
                    } else {
                        (200, serde_json::to_vec(&current).unwrap())
                    }
                }
                "/api/requests/7" => {
                    let mut exchange = crate::demo::exchange(7);
                    exchange.response.json = Some(serde_json::json!({"fresh_capture":true}));
                    (200, serde_json::to_vec(&exchange).unwrap())
                }
                "/api/requests/4" => (404, b"{}".to_vec()),
                _ => panic!("Unexpected inspector route {route}"),
            };
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    let root = std::env::temp_dir().join(format!(
        "jev-export-test-{}-{restart}-{unavailable}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("connection.json");
    let output = root.join("session.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&jev_protocol::Connection {
            version: PROTOCOL_VERSION,
            inspector_url: format!("http://{address}"),
            token: "synthetic-test-token".into(),
        })
        .unwrap(),
    )
    .unwrap();
    std::fs::write(&output, b"existing file stays intact on failure").unwrap();
    state.connection_file = config.clone();
    start_save(&mut state, output.clone(), 0);
    let until = Instant::now() + Duration::from_secs(5);
    while state.workspace.job.is_some() && Instant::now() < until {
        state.poll_job();
        std::thread::sleep(Duration::from_millis(2));
    }
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    assert!(state.workspace.job.is_none(), "export timed out");
    if restart {
        assert!(
            state
                .workspace
                .notice
                .as_ref()
                .is_some_and(|(m, e)| *e && m.contains("restarted"))
        );
        assert_eq!(
            std::fs::read(&output).unwrap(),
            b"existing file stays intact on failure"
        );
    } else {
        let saved = read(&output).unwrap();
        assert_eq!(saved.missing, vec![4]);
        assert_eq!(saved.snapshot.requests.len(), 2);
        assert_eq!(saved.last_received, unavailable);
        if unavailable {
            assert_eq!(
                saved.exchanges[&7].response.json,
                crate::demo::exchange(7).response.json
            );
            assert!(
                state
                    .workspace
                    .notice
                    .as_ref()
                    .unwrap()
                    .0
                    .contains("last-received")
            );
        } else {
            assert_eq!(
                saved.exchanges[&7].response.json.as_ref().unwrap()["fresh_capture"],
                true
            );
        }
    }
    std::fs::remove_file(config).unwrap();
    std::fs::remove_file(output).unwrap();
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn live_export_fetches_fresh_details_and_marks_evicted_captures() {
    exercise(false, false);
}
#[test]
fn live_export_aborts_on_relay_restart_without_overwriting_destination() {
    exercise(true, false);
}

#[test]
fn lost_connection_during_export_preserves_original_cached_snapshot() {
    exercise(false, true);
}

#[test]
fn disconnected_cached_sessions_save_reopen_and_resave_with_provenance() {
    for known_disconnection in [false, true] {
        let mut state = State::default();
        crate::demo::populate(&mut state);
        state.demo = false;
        state.connection_file = std::env::temp_dir().join(format!(
            "jev-missing-connection-{}-{known_disconnection}.json",
            std::process::id()
        ));
        assert!(!state.connection_file.exists());
        state.connection_error = known_disconnection.then(|| "Relay unavailable".into());
        state.workspace.cache.insert(7, crate::demo::exchange(7));
        state.workspace.starred.insert(7);
        let output = std::env::temp_dir().join(format!(
            "jev-cached-session-{}-{known_disconnection}.json",
            std::process::id()
        ));
        start_save(&mut state, output.clone(), 0);
        let until = Instant::now() + Duration::from_secs(5);
        while state.workspace.job.is_some() && Instant::now() < until {
            state.poll_job();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(state.workspace.job.is_none());
        let saved = read(&output).unwrap();
        assert!(saved.last_received);
        assert_eq!(saved.exchanges.len(), 2);
        assert_eq!(saved.missing.len(), 10);
        assert!(saved.starred.contains(&7));
        assert_eq!(
            saved.exchanges[&7].response.json,
            crate::demo::exchange(7).response.json
        );
        assert!(
            state
                .workspace
                .notice
                .as_ref()
                .unwrap()
                .0
                .contains("last-received snapshot")
        );
        let job = Arc::new(Job::default());
        *job.result.lock().unwrap() =
            Some(Ok(ResultValue::Loaded(Box::new(saved), output.clone())));
        state.workspace.job = Some(job);
        state.poll_job();
        assert!(state.workspace.last_received);
        assert!(
            state
                .workspace
                .notice
                .as_ref()
                .unwrap()
                .0
                .contains("last-received snapshot")
        );
        start_save(&mut state, output.clone(), 0);
        let until = Instant::now() + Duration::from_secs(5);
        while state.workspace.job.is_some() && Instant::now() < until {
            state.poll_job();
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(state.workspace.job.is_none());
        assert!(read(&output).unwrap().last_received);
        std::fs::remove_file(output).unwrap();
    }
}
