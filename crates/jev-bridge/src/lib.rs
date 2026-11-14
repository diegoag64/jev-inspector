pub mod capture;
pub mod store;

use axum::body::HttpBody;

use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::StreamExt;
use jev_protocol::{Connection, Outcome, Summary};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use capture::RawCapture;
use store::{Store, Work};

pub const REQUEST_LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Clone)]
pub struct Bridge {
    pub store: Arc<Mutex<Store>>,
    upstream: reqwest::Url,
    client: reqwest::Client,
    token: String,
    relay_addr: SocketAddr,
    inspector_addr: SocketAddr,
    capture: bool,
    worker: mpsc::SyncSender<Work>,
    permits: Arc<Semaphore>,
}

impl Bridge {
    pub fn new(
        upstream: &str,
        relay_addr: SocketAddr,
        inspector_addr: SocketAddr,
        token: String,
        capture: bool,
        history: usize,
        timeout: Duration,
    ) -> Result<Self, String> {
        if !relay_addr.ip().is_loopback() || !inspector_addr.ip().is_loopback() {
            return Err("Version 1 supports loopback listeners only.".into());
        }
        if !(1..=200).contains(&history) {
            return Err("History must contain 1 to 200 exchanges.".into());
        }
        if timeout.is_zero() || timeout > Duration::from_secs(600) {
            return Err("Timeout must be 1 to 600 seconds.".into());
        }
        if token.len() < 32 {
            return Err("Inspector token must contain at least 32 characters.".into());
        }
        let upstream = reqwest::Url::parse(upstream).map_err(|_| "Invalid upstream URL.")?;
        let host = upstream.host_str().unwrap_or("");
        let local = host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback());
        if !(upstream.scheme() == "https" || (upstream.scheme() == "http" && local))
            || !upstream.username().is_empty()
            || upstream.password().is_some()
            || upstream.query().is_some()
            || upstream.fragment().is_some()
            || upstream.path() != "/"
        {
            return Err("Upstream must be an HTTPS origin without credentials, path or query; HTTP is allowed only for a loopback mock.".into());
        }
        if local
            && [relay_addr.port(), inspector_addr.port()]
                .contains(&upstream.port_or_known_default().unwrap_or(0))
        {
            return Err("Upstream cannot point back to either bridge listener.".into());
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .connect_timeout(Duration::from_secs(10))
            .timeout(timeout)
            .build()
            .map_err(|_| "Cannot initialize upstream TLS/HTTP client.")?;
        let store = Arc::new(Mutex::new(Store::new(
            format!("http://{relay_addr}"),
            upstream.to_string(),
            capture,
            history,
        )));
        let worker = store::worker(store.clone());
        Ok(Self {
            store,
            upstream,
            client,
            token,
            relay_addr,
            inspector_addr,
            capture,
            worker,
            permits: Arc::new(Semaphore::new(64)),
        })
    }

    pub fn relay_router(&self) -> Router {
        Router::new()
            .route("/v1/systemone", post(relay))
            .route("/v1/models", get(relay))
            .layer(middleware::from_fn_with_state(self.clone(), relay_access))
            .with_state(self.clone())
    }
    pub fn inspector_router(&self) -> Router {
        Router::new()
            .route("/api/requests", get(snapshot))
            .route("/api/requests/{id}", get(detail))
            .layer(middleware::from_fn_with_state(
                self.clone(),
                inspector_access,
            ))
            .with_state(self.clone())
    }

    /// Serve the relay and inspector listeners until `shutdown` resolves, then let
    /// in-flight requests finish for up to five seconds. The CLI stops on Ctrl+C; the
    /// desktop app stops when it quits or the user picks another port.
    pub async fn serve(
        &self,
        relay: tokio::net::TcpListener,
        inspector: tokio::net::TcpListener,
        shutdown: impl std::future::Future<Output = ()>,
    ) -> std::io::Result<()> {
        let (stop, receiver) = tokio::sync::watch::channel(false);
        let mut relay_stop = receiver.clone();
        let mut inspector_stop = receiver;
        let servers = async {
            tokio::try_join!(
                axum::serve(relay, self.relay_router()).with_graceful_shutdown(async move {
                    let _ = relay_stop.changed().await;
                }),
                axum::serve(inspector, self.inspector_router()).with_graceful_shutdown(
                    async move {
                        let _ = inspector_stop.changed().await;
                    }
                )
            )
        };
        tokio::pin!(servers);
        tokio::select! {
            result = &mut servers => result.map(|_| ()),
            () = shutdown => {
                let _ = stop.send(true);
                let _ = tokio::time::timeout(Duration::from_secs(5), &mut servers).await;
                Ok(())
            }
        }
    }
}

/// A fresh bearer token for the inspector API.
pub fn new_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Atomically write the inspector connection file, readable by the owner only.
pub fn write_connection(path: &std::path::Path, connection: &Connection) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".connection-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(connection)?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}

/// Remove the connection file unless another relay instance has since replaced it.
pub fn remove_connection(path: &std::path::Path, token: &str) {
    if std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Connection>(&b).ok())
        .is_some_and(|current| current.token == token)
    {
        let _ = std::fs::remove_file(path);
    }
}

fn error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({"error": {"message": message, "source": "jev-bridge"}})),
    )
        .into_response()
}

fn local_request(headers: &HeaderMap, addr: SocketAddr) -> bool {
    if headers.contains_key("origin") || headers.contains_key("sec-fetch-site") {
        return false;
    }
    let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    host == addr.to_string() || host == format!("localhost:{}", addr.port())
}

async fn relay_access(State(state): State<Bridge>, request: Request, next: Next) -> Response {
    if !local_request(request.headers(), state.relay_addr) {
        return error(
            StatusCode::FORBIDDEN,
            "Only direct local backend clients are supported; browser origins are rejected.",
        );
    }
    next.run(request).await
}

async fn inspector_access(State(state): State<Bridge>, request: Request, next: Next) -> Response {
    if !local_request(request.headers(), state.inspector_addr) {
        return error(StatusCode::FORBIDDEN, "Local inspector access only.");
    }
    let authorized = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .is_some_and(|token| constant_time_equal(token.as_bytes(), state.token.as_bytes()));
    if !authorized {
        return error(
            StatusCode::UNAUTHORIZED,
            "Inspector authentication required.",
        );
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response
}

fn constant_time_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

async fn snapshot(State(state): State<Bridge>) -> impl IntoResponse {
    Json(state.store.lock().expect("capture store").snapshot())
}

async fn detail(State(state): State<Bridge>, Path(id): Path<u64>) -> Response {
    match state.store.lock().expect("capture store").get(id) {
        Some(exchange) => Json(exchange).into_response(),
        None => error(
            StatusCode::NOT_FOUND,
            "Exchange is no longer in the bounded history.",
        ),
    }
}

/// Keep end-to-end fields, including duplicate values. Never store this map.
pub fn forward_headers(input: &HeaderMap) -> HeaderMap {
    let mut output = input.clone();
    for value in input.get_all("connection") {
        if let Ok(value) = value.to_str() {
            for field in value.split(',') {
                output.remove(field.trim());
            }
        }
    }
    for field in [
        "host",
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    ] {
        output.remove(field);
    }
    output
}

fn header_text(headers: &HeaderMap, name: &str, limit: usize) -> String {
    headers
        .get(name)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .chars()
        .take(limit)
        .collect()
}

fn raw_capture(headers: &HeaderMap, enabled: bool) -> RawCapture {
    let expected_length = headers
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    RawCapture {
        enabled,
        content_type: header_text(headers, "content-type", 512),
        encoding: header_text(headers, "content-encoding", 80),
        expected_length,
        complete: expected_length == Some(0),
        ..RawCapture::default()
    }
}

struct Attempt {
    id: u64,
    start: Instant,
    finished: AtomicBool,
    request: Mutex<RawCapture>,
    response: Mutex<RawCapture>,
    state: Bridge,
}

impl Attempt {
    fn finish(&self, outcome: Outcome, note: Option<&str>) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        let request = std::mem::take(&mut *self.request.lock().expect("request capture"));
        let response = std::mem::take(&mut *self.response.lock().expect("response capture"));
        self.state
            .store
            .lock()
            .expect("capture store")
            .update(self.id, |entry| {
                entry.summary.outcome = outcome;
                entry.summary.duration_ms = Some(self.start.elapsed().as_secs_f64() * 1000.0);
                entry.summary.request_bytes = request.wire_bytes;
                entry.summary.response_bytes = response.wire_bytes;
                entry.summary.note = note.map(str::to_string);
                entry.request = request.metadata("Inspection is processing.");
                entry.response = response.metadata("Inspection is processing.");
            });
        if self
            .state
            .worker
            .try_send(Work {
                id: self.id,
                request,
                response,
            })
            .is_err()
        {
            let mut store = self.state.store.lock().expect("capture store");
            store.snapshot.capture_dropped += 1;
            store.snapshot.revision += 1;
            store.update(self.id, |entry| {
                for capture in [&mut entry.request, &mut entry.response] {
                    capture.notes = vec![
                        "Capture queue was full or unavailable. Forwarding was unaffected.".into(),
                    ];
                }
            });
        }
    }
}

struct Guard {
    attempt: Arc<Attempt>,
    _permit: OwnedSemaphorePermit,
}
impl Drop for Guard {
    fn drop(&mut self) {
        // HTTP can stop polling once Content-Length bytes have been produced.
        // Reaching that length is completion even without a final poll for None.
        let complete = self
            .attempt
            .response
            .lock()
            .expect("response capture")
            .complete;
        if complete {
            self.attempt.finish(Outcome::Complete, None);
        } else {
            self.attempt.finish(Outcome::Disconnected, Some("Client disconnected before the transfer completed. Upstream execution may have continued."));
        }
    }
}

async fn relay(State(state): State<Bridge>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let id = state.store.lock().expect("capture store").start(Summary {
        method: parts.method.to_string(),
        path: parts.uri.path().to_string(),
        started_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        ..Summary::default()
    });
    let permit = match state.permits.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            state
                .store
                .lock()
                .expect("capture store")
                .update(id, |entry| {
                    entry.summary.status = Some(503);
                    entry.summary.outcome = Outcome::Rejected;
                    entry.summary.note =
                        Some("Relay concurrency limit reached; upstream was not called.".into());
                });
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Relay concurrency limit reached.",
            );
        }
    };
    let mut request_capture = raw_capture(&parts.headers, state.capture);
    request_capture.complete |= body.is_end_stream();
    let attempt = Arc::new(Attempt {
        id,
        start: Instant::now(),
        finished: AtomicBool::new(false),
        request: Mutex::new(request_capture),
        response: Mutex::new(RawCapture {
            enabled: state.capture,
            ..RawCapture::default()
        }),
        state: state.clone(),
    });
    let guard = Guard {
        attempt: attempt.clone(),
        _permit: permit,
    };
    if parts
        .headers
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|n| n > REQUEST_LIMIT)
    {
        state
            .store
            .lock()
            .expect("capture store")
            .update(id, |entry| entry.summary.status = Some(413));
        attempt.finish(
            Outcome::Rejected,
            Some("Request exceeds the 32 MiB transport limit; upstream was not called."),
        );
        return error(StatusCode::PAYLOAD_TOO_LARGE, "Request exceeds 32 MiB.");
    }
    let request_attempt = attempt.clone();
    let request_stream: futures_util::stream::BoxStream<
        'static,
        Result<bytes::Bytes, std::io::Error>,
    > = Box::pin(async_stream::try_stream! {
        let mut stream = body.into_data_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(std::io::Error::other)?;
            let exceeded = {
                let mut raw = request_attempt.request.lock().expect("request capture");
                raw.observe(&chunk); raw.wire_bytes > REQUEST_LIMIT
            };
            if exceeded { Err(std::io::Error::other("Request transport limit exceeded"))?; }
            yield chunk;
        }
        request_attempt.request.lock().expect("request capture").complete = true;
    });
    let mut url = state.upstream.clone();
    url.set_path(parts.uri.path());
    url.set_query(parts.uri.query());
    let result = state
        .client
        .request(parts.method, url)
        .headers(forward_headers(&parts.headers))
        .body(reqwest::Body::wrap_stream(request_stream))
        .send()
        .await;
    let upstream = match result {
        Ok(upstream) => upstream,
        Err(error_value) => {
            let too_large =
                attempt.request.lock().expect("request capture").wire_bytes > REQUEST_LIMIT;
            let status = if too_large {
                StatusCode::PAYLOAD_TOO_LARGE
            } else if error_value.is_timeout() {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };
            state
                .store
                .lock()
                .expect("capture store")
                .update(id, |entry| entry.summary.status = Some(status.as_u16()));
            let message = if too_large {
                "Request transport limit exceeded after partial upload."
            } else if error_value.is_timeout() {
                "Upstream deadline exceeded; execution outcome is unknown."
            } else {
                "Upstream transport failed; execution outcome is unknown."
            };
            attempt.finish(
                if too_large {
                    Outcome::Rejected
                } else {
                    Outcome::UpstreamError
                },
                Some(message),
            );
            return error(status, message);
        }
    };
    let status = upstream.status();
    let headers = forward_headers(upstream.headers());
    let mut response_capture = raw_capture(upstream.headers(), state.capture);
    // HTTP statuses such as 204 have no body even without Content-Length.
    // Hyper can finish them without ever polling our response stream.
    response_capture.complete |= upstream.content_length() == Some(0);
    *attempt.response.lock().expect("response capture") = response_capture;
    let request_id = header_text(upstream.headers(), "x-typesafe-request-id", 200);
    state
        .store
        .lock()
        .expect("capture store")
        .update(id, |entry| {
            entry.summary.status = Some(status.as_u16());
            entry.summary.upstream_headers_ms =
                Some(attempt.start.elapsed().as_secs_f64() * 1000.0);
            entry.summary.request_id = (!request_id.is_empty()).then_some(request_id);
        });
    let response_stream: futures_util::stream::BoxStream<
        'static,
        Result<bytes::Bytes, std::io::Error>,
    > = Box::pin(async_stream::try_stream! {
        let _guard = guard;
        let mut stream = upstream.bytes_stream();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk) => {
                    attempt.response.lock().expect("response capture").observe(&chunk);
                    yield chunk;
                }
                Err(_) => {
                    attempt.finish(Outcome::UpstreamError, Some("Upstream response ended early; the received body is incomplete."));
                    Err(std::io::Error::other("Upstream response ended early"))?;
                }
            }
        }
        attempt.response.lock().expect("response capture").complete = true;
        attempt.finish(Outcome::Complete, None);
    });
    let mut response = Response::new(Body::from_stream(response_stream));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}
