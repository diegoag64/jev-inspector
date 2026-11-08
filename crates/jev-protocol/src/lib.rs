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
    /// Provider-reported usage, populated after response inspection completes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

impl TokenUsage {
    pub fn from_json(response: &Value) -> Option<Self> {
        let usage = response.get("usage")?.as_object()?;
        let value = Self {
            input_tokens: usage.get("input_tokens").and_then(Value::as_u64),
            output_tokens: usage.get("output_tokens").and_then(Value::as_u64),
        };
        (value.input_tokens.is_some() || value.output_tokens.is_some()).then_some(value)
    }
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

impl Capture {
    pub fn token_usage(&self) -> Option<TokenUsage> {
        self.complete.then_some(())?;
        TokenUsage::from_json(self.json.as_ref()?)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn usage_requires_nonnegative_integer_counts_and_a_complete_capture() {
        for value in [json!(-1), json!(1.5), json!("120"), Value::Null] {
            assert_eq!(
                TokenUsage::from_json(&json!({"usage":{"input_tokens":value}})),
                None
            );
        }
        let mut capture = Capture {
            complete: true,
            json: Some(json!({"usage":{"input_tokens":0,"output_tokens":null}})),
            ..Capture::default()
        };
        assert_eq!(
            capture.token_usage(),
            Some(TokenUsage {
                input_tokens: Some(0),
                output_tokens: None
            })
        );
        capture.complete = false;
        assert_eq!(capture.token_usage(), None);
        let mut old = serde_json::to_value(Summary::default()).unwrap();
        old.as_object_mut().unwrap().remove("usage");
        assert!(
            serde_json::from_value::<Summary>(old)
                .unwrap()
                .usage
                .is_none()
        );
    }
}
