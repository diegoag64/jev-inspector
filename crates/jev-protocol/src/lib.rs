//! Versioned, transport-independent inspector data. No API keys or raw headers.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connection {
    pub version: u32,
    pub inspector_url: String,
    pub token: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    #[default]
    Pending,
    Complete,
    Disconnected,
    UpstreamError,
    Rejected,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summary {
    pub id: u64,
    pub started_at_ms: u64,
    pub method: String,
    pub path: String,
    pub status: Option<u16>,
    pub outcome: Outcome,
    pub duration_ms: Option<f64>,
    pub upstream_headers_ms: Option<f64>,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Capture {
    pub content_type: String,
    pub encoding: String,
    pub wire_bytes: u64,
    pub captured_bytes: usize,
    pub complete: bool,
    /// Parsed inspection copy. Never used to construct the forwarded body.
    pub json: Option<Value>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Exchange {
    pub summary: Summary,
    pub request: Capture,
    pub response: Capture,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub instance_id: String,
    pub revision: u64,
    pub relay_url: String,
    pub upstream: String,
    pub capture_bodies: bool,
    pub history_limit: usize,
    pub total_requests: u64,
    pub evicted: u64,
    pub capture_dropped: u64,
    pub requests: Vec<Summary>,
}
