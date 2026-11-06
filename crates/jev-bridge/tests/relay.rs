use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Request,
    http::{HeaderValue, StatusCode},
    response::Response,
    routing::{get, post},
};
use jev_bridge::Bridge;
use jev_protocol::{Exchange, Outcome, Snapshot};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{net::TcpListener, task::JoinHandle};

const TOKEN: &str = "synthetic-inspector-token-at-least-32-characters";

struct Fixture {
    relay: String,
    inspector: String,
    state: Bridge,
    tasks: Vec<JoinHandle<()>>,
    client: reqwest::Client,
}

impl Fixture {
    async fn new(mock: Router, history: usize, capture: bool) -> Self {
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let relay = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let inspector = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let state = Bridge::new(
            &format!("http://{}", upstream.local_addr().unwrap()),
            relay.local_addr().unwrap(),
            inspector.local_addr().unwrap(),
            TOKEN.into(),
            capture,
            history,
            Duration::from_secs(5),
        )
        .unwrap();
        let relay_url = format!("http://{}", relay.local_addr().unwrap());
        let inspector_url = format!("http://{}", inspector.local_addr().unwrap());
        let relay_router = state.relay_router();
        let inspector_router = state.inspector_router();
        let tasks = vec![
            tokio::spawn(async move {
                axum::serve(upstream, mock).await.unwrap();
            }),
            tokio::spawn(async move {
                axum::serve(relay, relay_router).await.unwrap();
            }),
            tokio::spawn(async move {
                axum::serve(inspector, inspector_router).await.unwrap();
            }),
        ];
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .unwrap();
        Self {
            relay: relay_url,
            inspector: inspector_url,
            state,
            tasks,
            client,
        }
    }
    async fn snapshot(&self) -> Snapshot {
        self.client
            .get(format!("{}/api/requests", self.inspector))
            .bearer_auth(TOKEN)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }
    async fn settled(&self, id: u64) -> Exchange {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            let entry = self.state.store.lock().unwrap().get(id).unwrap();
            if entry.summary.outcome != Outcome::Pending
                && !entry.request.notes.iter().any(|s| s.contains("processing"))
            {
                return entry;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "exchange did not settle: {entry:?}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

#[tokio::test]
async fn empty_response_without_content_length_completes() {
    let mock = Router::new().route("/v1/models", get(|| async { StatusCode::NO_CONTENT }));
    let fixture = Fixture::new(mock, 10, true).await;
    let response = fixture
        .client
        .get(format!("{}/v1/models", fixture.relay))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.bytes().await.unwrap().is_empty());
    let entry = fixture.settled(1).await;
    assert_eq!(entry.summary.outcome, Outcome::Complete);
    assert!(entry.response.complete);
    assert_eq!(entry.summary.response_bytes, 0);
}

#[tokio::test]
async fn encoded_bytes_headers_status_and_request_id_survive_without_retries() {
    use std::io::Write;
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let input =
        br#"{"model":"probe","state":{"api_key":"do-not-retain","text":"hello"},"questions":{}}"#;
    let wire = zstd::stream::encode_all(input.as_slice(), 1).unwrap();
    let expected = wire.clone();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(br#"{"model":"probe","answers":{}}"#)
        .unwrap();
    let response_bytes = gzip.finish().unwrap();
    let expected_response = response_bytes.clone();
    let mock = Router::new().route(
        "/v1/systemone",
        post(move |request: Request| {
            let calls = count.clone();
            let expected = expected.clone();
            let response_bytes = response_bytes.clone();
            async move {
                calls.fetch_add(1, Ordering::Relaxed);
                assert_eq!(request.headers()["authorization"], "Bearer synthetic-key");
                assert_eq!(request.headers()["content-encoding"], "zstd");
                assert_eq!(request.headers()["typesafe-danger-zone-images"], "true");
                assert!(!request.headers().contains_key("x-remove-me"));
                assert_eq!(
                    to_bytes(request.into_body(), 1024 * 1024)
                        .await
                        .unwrap()
                        .as_ref(),
                    expected
                );
                let mut response = Response::new(Body::from(response_bytes));
                *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                for (key, value) in [
                    ("content-type", "application/json"),
                    ("content-encoding", "gzip"),
                    ("x-typesafe-request-id", "request-probe"),
                    ("retry-after", "7"),
                ] {
                    response
                        .headers_mut()
                        .insert(key, HeaderValue::from_static(value));
                }
                response
            }
        }),
    );
    let fixture = Fixture::new(mock, 10, true).await;
    let response = fixture
        .client
        .post(format!("{}/v1/systemone", fixture.relay))
        .header("authorization", "Bearer synthetic-key")
        .header("content-type", "application/json")
        .header("content-encoding", "zstd")
        .header("typesafe-danger-zone-images", "true")
        .header("connection", "x-remove-me")
        .header("x-remove-me", "hop value")
        .body(wire)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 429);
    assert_eq!(response.headers()["retry-after"], "7");
    assert_eq!(response.headers()["content-encoding"], "gzip");
    assert_eq!(response.bytes().await.unwrap().as_ref(), expected_response);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    let entry = fixture.settled(1).await;
    assert_eq!(entry.summary.outcome, Outcome::Complete);
    assert_eq!(entry.summary.request_id.as_deref(), Some("request-probe"));
    assert_eq!(
        entry.request.json.as_ref().unwrap()["state"]["api_key"],
        "[redacted]"
    );
    assert!(
        !serde_json::to_string(&entry)
            .unwrap()
            .contains("do-not-retain")
    );
    assert!(
        !serde_json::to_string(&entry)
            .unwrap()
            .contains("synthetic-key")
    );
}

#[tokio::test]
async fn capture_budget_does_not_truncate_forwarded_bodies_and_history_is_bounded() {
    let mock = Router::new().route(
        "/v1/systemone",
        post(|request: Request| async {
            let body = to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
            Response::builder()
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap()
        }),
    );
    let fixture = Fixture::new(mock, 2, true).await;
    let payload = serde_json::to_vec(&serde_json::json!({"state":"x".repeat(200_000)})).unwrap();
    for id in 1..=3 {
        let response = fixture
            .client
            .post(format!("{}/v1/systemone", fixture.relay))
            .header("content-type", "application/json")
            .body(payload.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(response.bytes().await.unwrap().as_ref(), payload);
        let entry = fixture.settled(id).await;
        assert!(entry.request.json.is_none());
        assert!(entry.request.notes[0].contains("64 KiB"));
        assert_eq!(entry.summary.request_bytes, payload.len() as u64);
    }
    let snapshot = fixture.snapshot().await;
    assert_eq!(snapshot.requests.len(), 2);
    assert_eq!(snapshot.evicted, 1);
    assert_eq!(snapshot.total_requests, 3);
    assert_eq!(
        fixture
            .client
            .get(format!("{}/api/requests/1", fixture.inspector))
            .bearer_auth(TOKEN)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

#[tokio::test]
async fn inspector_auth_origin_checks_and_fixed_routes_reject_untrusted_access() {
    let fixture = Fixture::new(Router::new(), 10, false).await;
    let url = format!("{}/api/requests", fixture.inspector);
    assert_eq!(fixture.client.get(&url).send().await.unwrap().status(), 401);
    assert_eq!(
        fixture
            .client
            .get(&url)
            .bearer_auth("wrong")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        fixture
            .client
            .get(&url)
            .bearer_auth(TOKEN)
            .header("origin", "https://example.com")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        fixture
            .client
            .get(&url)
            .bearer_auth(TOKEN)
            .header("host", "evil.example")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        fixture
            .client
            .post(format!("{}/v1/systemone", fixture.relay))
            .header("origin", "https://example.com")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        fixture
            .client
            .get(format!("{}/anything", fixture.relay))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        fixture
            .client
            .get(&url)
            .bearer_auth(TOKEN)
            .send()
            .await
            .unwrap()
            .headers()["cache-control"],
        "no-store"
    );
}

#[tokio::test]
async fn redirects_are_returned_without_following_or_leaking_credentials() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let mock = Router::new()
        .route(
            "/v1/systemone",
            post(|| async {
                Response::builder()
                    .status(307)
                    .header("location", "/steal")
                    .body(Body::empty())
                    .unwrap()
            }),
        )
        .route(
            "/steal",
            post(move || {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    "wrong"
                }
            }),
        );
    let fixture = Fixture::new(mock, 10, false).await;
    let response = fixture
        .client
        .post(format!("{}/v1/systemone", fixture.relay))
        .bearer_auth("synthetic")
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 307);
    response.bytes().await.unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let entry = fixture.settled(1).await;
    assert!(entry.request.json.is_none());
    assert!(entry.request.notes[0].contains("disabled"));
}

#[tokio::test]
async fn first_response_chunk_arrives_before_upstream_finishes_and_disconnect_is_recorded() {
    let mock = Router::new().route(
        "/v1/systemone",
        post(|| async {
            let stream = async_stream::stream! {
                yield Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"first"));
                tokio::time::sleep(Duration::from_secs(2)).await;
                yield Ok(bytes::Bytes::from_static(b"last"));
            };
            Response::new(Body::from_stream(stream))
        }),
    );
    let fixture = Fixture::new(mock, 10, true).await;
    let start = std::time::Instant::now();
    let mut response = fixture
        .client
        .post(format!("{}/v1/systemone", fixture.relay))
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(response.chunk().await.unwrap().unwrap(), "first");
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "relay buffered the whole response"
    );
    drop(response);
    let entry = fixture.settled(1).await;
    assert_eq!(entry.summary.outcome, Outcome::Disconnected);
    assert!(!entry.response.complete);
}

#[tokio::test]
async fn parallel_calls_complete_without_an_inspector_subscriber() {
    let mock = Router::new().route(
        "/v1/models",
        get(|| async { axum::Json(serde_json::json!({"models":[]})) }),
    );
    let fixture = Fixture::new(mock, 20, true).await;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let client = fixture.client.clone();
        let url = format!("{}/v1/models", fixture.relay);
        tasks.spawn(async move {
            assert_eq!(
                client
                    .get(url)
                    .send()
                    .await
                    .unwrap()
                    .json::<serde_json::Value>()
                    .await
                    .unwrap()["models"],
                serde_json::json!([])
            );
        });
    }
    while let Some(result) = tasks.join_next().await {
        result.unwrap();
    }
    for id in 1..=16 {
        assert_eq!(fixture.settled(id).await.summary.outcome, Outcome::Complete);
    }
    assert_eq!(fixture.snapshot().await.total_requests, 16);
}

#[test]
fn listener_and_upstream_configuration_prevents_accidental_exposure_and_loops() {
    let make = |upstream: &str, bind: &str| {
        Bridge::new(
            upstream,
            bind.parse().unwrap(),
            "127.0.0.1:8788".parse().unwrap(),
            TOKEN.into(),
            false,
            100,
            Duration::from_secs(10),
        )
    };
    assert!(make("https://api.typesafe.ai", "0.0.0.0:8787").is_err());
    assert!(make("http://example.com", "127.0.0.1:8787").is_err());
    assert!(make("https://user:secret@example.com", "127.0.0.1:8787").is_err());
    assert!(make("https://api.typesafe.ai/v1", "127.0.0.1:8787").is_err());
    assert!(make("http://localhost:8787", "127.0.0.1:8787").is_err());
}
