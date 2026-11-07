use crate::State;
use jev_protocol::{Capture, Exchange, Outcome, PROTOCOL_VERSION, Snapshot, Summary};
use serde_json::json;

pub fn exchange(id: u64) -> Exchange {
    let status = match id {
        7 => 429,
        4 => 401,
        _ => 200,
    };
    let request = json!({"model":"jev-latest", "state":{"message":"My invoice was charged twice.","channel":"support"},
        "questions":{"department":{"type":"choice","instructions":"Which team should handle this?","criteria":{"billing":"Payments and invoices","technical":"Product issues","sales":"Plans and pricing"}},
        "needs_follow_up":{"type":"noul","instructions":"Does this require a response?"}}});
    let response = if status == 200 {
        json!({"model":"jev-demo", "answers":{
        "department":{"type":"choice","choice":"billing","probabilities":{"billing":0.94,"technical":0.04,"sales":0.02},"confidence":0.91},
        "needs_follow_up":{"type":"noul","noul":0.98}},"usage":{"input_tokens":186,"output_tokens":12}})
    } else {
        json!({"error":{"message":if status==429 {"Synthetic rate limit"} else {"Synthetic authentication failure"}}})
    };
    Exchange {
        summary: Summary {
            id,
            started_at_ms: 1793980800000 + id * 1000,
            method: "POST".into(),
            path: "/v1/systemone".into(),
            status: Some(status),
            outcome: Outcome::Complete,
            duration_ms: Some(108.3 + id as f64 * 13.7),
            upstream_headers_ms: Some(106.2 + id as f64 * 13.7),
            request_id: Some(format!("demo-request-{id:03}")),
            model: Some("jev-demo".into()),
            request_bytes: 428,
            response_bytes: 316,
            note: None,
        },
        request: Capture {
            content_type: "application/json".into(),
            wire_bytes: 428,
            captured_bytes: 428,
            complete: true,
            json: Some(request),
            ..Capture::default()
        },
        response: Capture {
            content_type: "application/json".into(),
            wire_bytes: 316,
            captured_bytes: 316,
            complete: true,
            json: Some(response),
            ..Capture::default()
        },
    }
}

pub fn populate(state: &mut State) {
    state.demo = true;
    state.snapshot = Snapshot {
        version: PROTOCOL_VERSION,
        instance_id: "demo".into(),
        revision: 1,
        relay_url: "Offline demo · no inference calls".into(),
        upstream: "Synthetic fixture".into(),
        capture_bodies: true,
        history_limit: 100,
        total_requests: 12,
        requests: (1..=12).rev().map(|id| exchange(id).summary).collect(),
        ..Snapshot::default()
    };
    state.filter_rows();
    state.select(12);
}

/// Deterministic renderer states. These never start the feed or load live preferences.
pub fn scenario(s: &mut State, name: &str) -> Result<(), String> {
    if name == "nested-properties" {
        crate::showcase::populate(s);
        crate::showcase::select_example(s, "support-routing")?;
        let mut detail = s.detail.clone().unwrap();
        let nested: serde_json::Map<String, serde_json::Value> = (0..24)
            .map(|i| (format!("property{i:02}"), serde_json::json!(i)))
            .collect();
        detail.request.json.as_mut().unwrap()["state"] = serde_json::json!({"nested": nested});
        let bytes = serde_json::to_vec(&detail.request.json).unwrap().len();
        detail.request.wire_bytes = bytes as u64;
        detail.request.captured_bytes = bytes;
        detail.summary.request_bytes = bytes as u64;
        if let Some(summary) = s
            .snapshot
            .requests
            .iter_mut()
            .find(|v| v.id == detail.summary.id)
        {
            *summary = detail.summary.clone();
        }
        s.workspace.cache.insert(detail.summary.id, detail.clone());
        s.set_detail(detail);
        return Ok(());
    }
    if let Some(slug) = name.strip_prefix("example-") {
        crate::showcase::populate(s);
        return crate::showcase::select_example(s, slug);
    }
    populate(s);
    s.offline_fixture = true;
    s.workspace.cache = (1..=12).map(|id| (id, exchange(id))).collect();
    match name {
        "loading" | "loading-request" | "loading-response" | "loading-diagram" | "loading-compare" | "detail-failed" | "disconnected-empty" => {
            s.demo = false;
            s.connected = true;
            s.detail = None;
            s.workspace.cache.remove(&12);
            s.tab = match name {
                "loading-request" => 1,
                "loading-response" => 2,
                "loading-diagram" => 3,
                "loading-compare" => 4,
                _ => 0,
            };
            if name == "detail-failed" {
                s.detail_error = Some("Inspector returned HTTP 404. The relay may have restarted or this exchange was evicted.".into());
            } else if name == "disconnected-empty" {
                s.connected = false;
                s.connection_error = Some("The local relay is not responding. Start jev-bridge or check its connection file.".into());
            }
        }
        "pending-response" | "empty-response" => {
            let mut current = exchange(12);
            current.response = Capture::default();
            current.summary.response_bytes = 0;
            if name == "pending-response" {
                current.summary.outcome = Outcome::Pending;
                current.summary.status = None;
                current.summary.duration_ms = None;
                current.summary.upstream_headers_ms = None;
            } else {
                current.summary.status = Some(204);
                current.response.complete = true;
            }
            s.snapshot.requests[0] = current.summary.clone();
            s.set_detail(current);
            s.filter_rows();
            s.tab = 2;
        }
        "first-use" | "saved-empty" => {
            s.snapshot.requests.clear();
            s.snapshot.total_requests = 0;
            s.workspace.cache.clear();
            s.demo = false;
            s.connected = true;
            s.snapshot.relay_url = "http://127.0.0.1:8787".into();
            if name == "saved-empty" {
                s.workspace.session_name = Some("empty-session.jev.json".into());
            }
            s.filter_rows();
        }
        "no-matches" => {
            s.filter = rune::TextEdit::new("model:nonexistent");
            s.filter_rows();
        }
        "compare-unset" => s.tab = 4,
        "saved" | "summary-only" => {
            s.workspace.session_name = Some("support-investigation.jev.json".into());
            if name == "summary-only" {
                s.workspace.cache.remove(&12);
                s.selected = None;
                s.select(12);
            }
        }
        "disconnected" => {
            s.demo = false;
            s.connection_error = Some("Relay unavailable. Showing the last received requests; reconnecting automatically.".into());
            s.detail_error = Some("The relay is disconnected.".into());
        }
        "paused" => {
            s.demo = false;
            s.connected = true;
            s.workspace.paused = true;
            let mut pending = s.snapshot.clone();
            pending.total_requests += 3;
            s.workspace.pending = Some(pending);
        }
        "missing-compare" => {
            let mut baseline = exchange(7);
            baseline.request.json = None;
            baseline.response.json = None;
            let mut current = exchange(12);
            current.request.json = None;
            current.response.json = None;
            s.workspace.comparison.baseline = Some(baseline);
            s.set_detail(current);
            s.tab = 4;
        }
        "many-questions-6" | "many-questions-12" | "many-questions-24" => {
            let count: usize = name.rsplit('-').next().unwrap().parse().unwrap();
            let mut current = exchange(12);
            let mut questions = serde_json::Map::new();
            let mut answers = serde_json::Map::new();
            for i in 0..count {
                let key = format!("question_{i:02}_customer_support_triage_and_resolution");
                questions.insert(key.clone(), json!({"type":"noul", "instructions":"Does this need review?"}));
                answers.insert(key, json!({"type":"noul", "noul":0.12, "confidence":0.89}));
            }
            current.request.json = Some(json!({"questions":questions, "state":{"message":"Synthetic multi-question example"}}));
            current.response.json = Some(json!({"answers":answers}));
            s.set_detail(current);
            s.tab = 3;
        }
        "probabilities-13" | "probabilities-40" => {
            let count: usize = name.rsplit('-').next().unwrap().parse().unwrap();
            let mut current = exchange(12);
            let probabilities: serde_json::Map<String, serde_json::Value> = (0..count)
                .map(|i| (format!("category_{i:02}"), json!(1.0 / count as f64))).collect();
            current.response.json = Some(json!({"answers":{"department":{"type":"choice", "choice":format!("category_{:02}",count-1), "confidence":0.89, "probabilities":probabilities}}}));
            s.set_detail(current);
            s.tab = 0;
        }
        "saved-pending" | "saved-partial" | "disconnected-pending" => {
            scenario(s, "pending-response")?;
            if name == "disconnected-pending" {
                s.demo = false;
                s.connected = false;
                s.connection_error = Some("The local relay is not responding.".into());
            } else {
                s.workspace.session_name = Some("incomplete-capture.jev.json".into());
                if name == "saved-partial" {
                    let mut current = s.detail.clone().unwrap();
                    current.response.json = Some(json!({"partial": true, "message": "Inspection data available when saved"}));
                    s.set_detail(current);
                }
            }
        }
        "compare-tail" | "compare-line-tail" => {
            let mut baseline = exchange(7);
            let mut current = exchange(12);
            let prefix = if name == "compare-tail" { json!("x".repeat(17_000)) } else { json!((0..1000).collect::<Vec<_>>()) };
            baseline.response.json = Some(json!({"a_prefix":prefix, "z_tail":1}));
            current.response.json = Some(json!({"a_prefix":prefix, "z_tail":2}));
            s.workspace.comparison.baseline = Some(baseline);
            s.set_detail(current);
            s.tab = 4;
        }
        "relay-restarted" => {
            s.select(4); s.star();
            s.workspace.comparison.baseline = s.detail.clone();
            let mut snapshot=s.snapshot.clone(); snapshot.instance_id="restart-example".into(); s.apply_snapshot(snapshot);
        }
        "setup-disconnected" => {
            scenario(s, "first-use")?;
            s.connected = false;
            s.connection_error = Some(format!("No relay connection at {}", s.connection_file.display()));
        }
        "long-json" => {
            let mut current = exchange(12);
            current.response.json = Some(
                json!({"a_long_value": "long scalar ".repeat(120), "records": (0..2000).map(|i| json!({"id":i,"department":"billing","message":"Synthetic captured record"})).collect::<Vec<_>>()}),
            );
            let bytes = serde_json::to_vec(current.response.json.as_ref().unwrap()).unwrap().len() as u64;
            current.summary.response_bytes = bytes;
            current.response.wire_bytes = bytes;
            current.response.captured_bytes = bytes as usize;
            s.set_detail(current);
            s.tab = 2;
        }
        "invalid-query" => {
            s.filter = rune::TextEdit::new("status:200ms");
            s.filter_rows();
        }
        "export-progress" => {
            let job = std::sync::Arc::new(crate::session::Job::default());
            job.total.store(12, std::sync::atomic::Ordering::Relaxed);
            job.done.store(4, std::sync::atomic::Ordering::Relaxed);
            s.workspace.job = Some(job);
        }
        "export-cancelled" => s.workspace.notice = Some(("File operation cancelled".into(), false)),
        "export-failed" => s.workspace.notice = Some(("Could not save session: the destination is unavailable. Choose another location and try again; your current investigation is unchanged.".into(), true)),
        "long-labels" => {
            let mut current = exchange(12);
            current.summary.path = format!("/v1/systemone?trace={}", "long-request-path-".repeat(30));
            current.summary.model = Some("customer-service-model-with-a-long-version-and-deployment-name-".repeat(5));
            current.summary.request_id = Some("upstream-request-id-with-long-correlation-context-".repeat(6));
            s.snapshot.requests[0] = current.summary.clone();
            s.workspace.session_name = Some(format!("investigation-{}session.jev.json", "customer-support-".repeat(20)));
            s.set_detail(current);
            s.filter_rows();
        }
        _ => {
            return Err(format!(
                "Unknown fixture '{name}'. See demo::scenario for supported fixture names."
            ));
        }
    }
    Ok(())
}
