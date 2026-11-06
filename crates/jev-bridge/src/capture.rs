use std::io::{Cursor, Read};

use bytes::Bytes;
use jev_protocol::Capture;
use serde_json::Value;

pub const WIRE_CAPTURE_LIMIT: usize = 64 * 1024;
pub const DECODE_LIMIT: usize = 128 * 1024;

#[derive(Default)]
pub struct RawCapture {
    pub content_type: String,
    pub encoding: String,
    pub bytes: Vec<u8>,
    pub wire_bytes: u64,
    pub complete: bool,
    pub expected_length: Option<u64>,
    pub enabled: bool,
}

impl RawCapture {
    pub fn observe(&mut self, bytes: &[u8]) {
        self.wire_bytes += bytes.len() as u64;
        if self.expected_length == Some(self.wire_bytes) {
            self.complete = true;
        }
        if self.enabled {
            let take = bytes
                .len()
                .min(WIRE_CAPTURE_LIMIT.saturating_sub(self.bytes.len()));
            self.bytes.extend_from_slice(&bytes[..take]);
        }
    }

    pub fn metadata(&self, note: &str) -> Capture {
        Capture {
            content_type: self.content_type.clone(),
            encoding: self.encoding.clone(),
            wire_bytes: self.wire_bytes,
            captured_bytes: self.bytes.len(),
            complete: self.complete,
            json: None,
            notes: vec![note.into()],
        }
    }
}

fn limited_read(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    reader
        .take(DECODE_LIMIT as u64 + 1)
        .read_to_end(&mut result)
        .map_err(|_| "Cannot decode this body.".to_string())?;
    if result.len() > DECODE_LIMIT {
        return Err("Decoded body exceeds the 128 KiB inspection budget.".into());
    }
    Ok(result)
}

fn decode(raw: &RawCapture) -> Result<Vec<u8>, String> {
    match raw.encoding.trim().to_ascii_lowercase().as_str() {
        "" | "identity" => Ok(raw.bytes.clone()),
        "gzip" | "x-gzip" => limited_read(flate2::read::MultiGzDecoder::new(raw.bytes.as_slice())),
        "zstd" => {
            let mut decoder = zstd::stream::read::Decoder::new(Cursor::new(&raw.bytes))
                .map_err(|_| "Invalid zstd body.".to_string())?;
            decoder
                .window_log_max(23)
                .map_err(|_| "Cannot limit zstd window.".to_string())?;
            limited_read(decoder)
        }
        _ => Err("This content encoding is forwarded but not decoded by the inspector.".into()),
    }
}

/// Redaction is intentionally a documented, best-effort key policy, not a claim
/// that arbitrary prose or application-specific sensitive data is safe to retain.
fn redact(value: &mut Value, count: &mut usize) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let normalized: String = key
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect();
                if matches!(
                    normalized.as_str(),
                    "authorization"
                        | "cookie"
                        | "setcookie"
                        | "password"
                        | "passwd"
                        | "secret"
                        | "apikey"
                        | "accesstoken"
                        | "refreshtoken"
                        | "token"
                ) {
                    *child = Value::String("[redacted]".into());
                    *count += 1;
                } else {
                    redact(child, count);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(|v| redact(v, count)),
        _ => {}
    }
}

fn within_node_budget(value: &Value, remaining: &mut usize) -> bool {
    if *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    match value {
        Value::Object(map) => map.values().all(|v| within_node_budget(v, remaining)),
        Value::Array(values) => values.iter().all(|v| within_node_budget(v, remaining)),
        _ => true,
    }
}

pub async fn inspect(raw: RawCapture) -> Capture {
    let mut capture = raw.metadata("");
    capture.notes.clear();
    if !raw.enabled {
        capture.notes.push(
            "Body capture is disabled. Start the relay with --capture-bodies to inspect payloads."
                .into(),
        );
        return capture;
    }
    if !raw.complete {
        capture.notes.push(
            "Transfer ended before this body was fully observed; no partial JSON is displayed."
                .into(),
        );
        return capture;
    }
    if raw.wire_bytes > raw.bytes.len() as u64 {
        capture
            .notes
            .push("Body exceeds the 64 KiB wire capture budget. Forwarding continued.".into());
        return capture;
    }
    if raw.bytes.is_empty() {
        return capture;
    }
    let decoded = match decode(&raw) {
        Ok(value) => value,
        Err(note) => {
            capture.notes.push(note);
            return capture;
        }
    };
    let json_bytes = if raw
        .content_type
        .to_ascii_lowercase()
        .starts_with("multipart/form-data")
    {
        let boundary = match multer::parse_boundary(&raw.content_type) {
            Ok(boundary) => boundary,
            Err(_) => {
                capture
                    .notes
                    .push("Multipart boundary is unavailable.".into());
                return capture;
            }
        };
        let stream =
            futures_util::stream::once(async { Ok::<_, std::io::Error>(Bytes::from(decoded)) });
        let mut multipart = multer::Multipart::new(stream, boundary);
        let mut request = None;
        let mut assets = 0;
        loop {
            match multipart.next_field().await {
                Ok(Some(field)) => {
                    let is_request = field.name() == Some("request");
                    match field.bytes().await {
                        Ok(bytes) if is_request && request.is_none() => {
                            request = Some(bytes.to_vec())
                        }
                        Ok(_) => assets += 1,
                        Err(_) => {
                            capture.notes.push("Multipart inspection failed.".into());
                            return capture;
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    capture.notes.push("Multipart inspection failed.".into());
                    return capture;
                }
            }
        }
        capture.notes.push(format!(
            "Multipart request: {assets} binary part(s) forwarded; image previews are not retained."
        ));
        match request {
            Some(bytes) => bytes,
            None => {
                capture.notes.push("No JSON request part found.".into());
                return capture;
            }
        }
    } else if raw.content_type.to_ascii_lowercase().contains("json") {
        decoded
    } else {
        capture
            .notes
            .push("Non-JSON body forwarded; raw text and binary contents are not retained.".into());
        return capture;
    };
    match serde_json::from_slice::<Value>(&json_bytes) {
        Ok(mut value) => {
            if !within_node_budget(&value, &mut 4096) {
                capture.notes.push(
                    "JSON exceeds the 4,096-node inspection budget. Forwarding was unchanged."
                        .into(),
                );
                return capture;
            }
            let mut redactions = 0;
            redact(&mut value, &mut redactions);
            if let Some(assets) = value.get_mut("assets").and_then(Value::as_object_mut) {
                for asset in assets.values_mut() {
                    if let Some(data) = asset.get_mut("data") {
                        *data = Value::String("[inline image data omitted]".into());
                        redactions += 1;
                    }
                }
            }
            if redactions > 0 {
                capture.notes.push(format!(
                    "{redactions} sensitive or binary field(s) removed from this inspection copy."
                ));
            }
            capture.json = Some(value);
        }
        Err(_) => capture.notes.push(
            "Body is not valid JSON within the parser's nesting limit. Forwarding was unchanged."
                .into(),
        ),
    }
    capture
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raw(bytes: Vec<u8>, encoding: &str) -> RawCapture {
        RawCapture {
            content_type: "application/json".into(),
            encoding: encoding.into(),
            wire_bytes: bytes.len() as u64,
            bytes,
            enabled: true,
            complete: true,
            expected_length: None,
        }
    }
    #[tokio::test]
    async fn compressed_inspection_redacts_only_the_copy_and_limits_expansion() {
        let bytes = br#"{"state":{"api_key":"secret-value","message":"hello"}}"#;
        let compressed = zstd::stream::encode_all(bytes.as_slice(), 1).unwrap();
        let capture = inspect(raw(compressed, "zstd")).await;
        assert_eq!(capture.json.unwrap()["state"]["api_key"], "[redacted]");
        assert!(bytes.windows(12).any(|w| w == b"secret-value"));
        let oversized =
            zstd::stream::encode_all(vec![b' '; DECODE_LIMIT + 1].as_slice(), 1).unwrap();
        let capture = inspect(raw(oversized, "zstd")).await;
        assert!(capture.json.is_none());
        assert!(capture.notes[0].contains("128 KiB"));
    }
    #[tokio::test]
    async fn truncated_and_invalid_bodies_do_not_become_fake_json() {
        let mut input = raw(b"{}".to_vec(), "");
        input.wire_bytes = 100000;
        assert!(inspect(input).await.json.is_none());
        assert!(inspect(raw(b"not json".to_vec(), "")).await.json.is_none());
    }
}
