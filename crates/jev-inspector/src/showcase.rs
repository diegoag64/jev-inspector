use super::*;
use jev_protocol::{Capture, PROTOCOL_VERSION};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Deserialize)]
struct Catalog {
    version: u32,
    cases: Vec<Example>,
}

#[derive(Deserialize)]
struct Example {
    id: String,
    title: String,
    method: String,
    path: String,
    request: Option<Value>,
    status: u16,
    response: Option<Value>,
    response_text: Option<String>,
    content_type: Option<String>,
    delay_ms: u64,
}

struct Item {
    id: u64,
    slug: String,
    title: String,
    tab: u8,
}

pub struct Playback {
    items: Vec<Item>,
    pub playing: bool,
    next_at: Option<f64>,
    reveal: bool,
}

fn capture(value: Option<Value>, text: Option<&str>, content_type: &str) -> Capture {
    let bytes = text.map_or_else(
        || {
            value
                .as_ref()
                .map_or(0, |v| serde_json::to_vec(v).unwrap().len())
        },
        str::len,
    );
    Capture {
        content_type: content_type.into(),
        encoding: String::new(),
        wire_bytes: bytes as u64,
        captured_bytes: bytes,
        complete: true,
        json: value,
        notes: if text.is_some() {
            vec!["The response is not JSON; no parsed inspection copy is available.".into()]
        } else {
            vec![]
        },
    }
}

pub fn populate(s: &mut State) {
    let catalog: Catalog =
        serde_json::from_str(include_str!("../../../examples/demo-requests.json"))
            .expect("valid embedded demo catalog");
    assert_eq!(catalog.version, 1);
    s.demo = true;
    s.offline_fixture = true;
    s.workspace = workspace::Workspace::default();
    s.selected = None;
    s.detail = None;
    s.detail_error = None;
    s.errors_only = false;
    s.filter = TextEdit::default();
    let mut items = Vec::new();
    let mut summaries = Vec::new();
    for (index, example) in catalog.cases.into_iter().enumerate() {
        let id = index as u64 + 1;
        let request = capture(example.request, None, "application/json");
        let response = capture(
            example.response,
            example.response_text.as_deref(),
            example
                .content_type
                .as_deref()
                .unwrap_or("application/json"),
        );
        let summary = Summary {
            id,
            started_at_ms: 1_793_980_800_000 + id * 3000,
            method: example.method,
            path: example.path,
            status: Some(example.status),
            outcome: Outcome::Complete,
            duration_ms: Some(example.delay_ms as f64),
            upstream_headers_ms: Some((example.delay_ms as f64 - 4.0).max(0.0)),
            request_id: Some(format!("demo-{}", example.id)),
            model: request
                .json
                .as_ref()
                .and_then(|v| v["model"].as_str())
                .map(str::to_owned),
            request_bytes: request.wire_bytes,
            response_bytes: response.wire_bytes,
            note: None,
        };
        let tab = match example.id.as_str() {
            "fraud-check" | "invoice-box" | "many-choices" | "support-routing" => 3,
            "inventory" | "unicode" => 1,
            "models" | "empty" | "non-json" => 2,
            _ => 0,
        };
        items.push(Item {
            id,
            slug: example.id,
            title: example.title,
            tab,
        });
        summaries.push(summary.clone());
        s.workspace.cache.insert(
            id,
            Exchange {
                summary,
                request,
                response,
            },
        );
    }
    s.snapshot = Snapshot {
        version: PROTOCOL_VERSION,
        instance_id: "varied-offline-showcase".into(),
        revision: 1,
        relay_url: "Offline examples".into(),
        upstream: "Synthetic examples".into(),
        capture_bodies: true,
        history_limit: 100,
        total_requests: summaries.len() as u64,
        requests: summaries,
        ..Snapshot::default()
    };
    s.filter_rows();
    s.tab = 0;
    s.showcase = Some(Playback {
        items,
        playing: false,
        next_at: None,
        reveal: true,
    });
}

pub fn select_example(s: &mut State, slug: &str) -> Result<(), String> {
    let item = s
        .showcase
        .as_ref()
        .and_then(|p| p.items.iter().find(|e| e.slug == slug))
        .map(|e| (e.id, e.tab))
        .ok_or_else(|| format!("Unknown example '{slug}'"))?;
    s.select(item.0);
    s.showcase.as_mut().unwrap().reveal = true;
    s.tab = item.1;
    s.request_text.tree = slug == "inventory";
    Ok(())
}

pub fn stop(s: &mut State) {
    if let Some(playback) = &mut s.showcase {
        playback.playing = false;
        playback.next_at = None;
    }
}

pub fn tick(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    let Some(mut playback) = s.showcase.take() else {
        return false;
    };
    let mut changed = false;
    if playback.reveal {
        if let Some(id) = s.selected {
            s.table.reveal(id.to_string(), tx);
        }
        playback.reveal = false;
        changed = true;
    }
    if !playback.playing
        || s.workspace.session_name.is_some()
        || s.workspace.help
        || s.workspace.export
        || s.visible.len() != playback.items.len()
        || s.filter.focused
        || s.request_text.search.focused
        || s.response_text.search.focused
    {
        changed |= playback.playing;
        playback.playing = false;
        playback.next_at = None;
        s.showcase = Some(playback);
        return changed;
    }
    if playback.next_at.is_none() {
        playback.next_at = Some(now + 3.2);
        tx.wake_after(Duration::from_millis(3200));
        changed = true;
    } else if now >= playback.next_at.unwrap() {
        let index = playback
            .items
            .iter()
            .position(|item| Some(item.id) == s.selected)
            .unwrap_or(0);
        let next = if index == 0 {
            playback.items.len() - 1
        } else {
            index - 1
        };
        let item = &playback.items[next];
        s.select(item.id);
        s.tab = item.tab;
        s.request_text.tree = item.slug == "inventory";
        s.response_text.tree = false;
        s.table.reveal(item.id.to_string(), tx);
        playback.next_at = Some(now + 3.2);
        tx.wake_after(Duration::from_millis(3200));
        tx.animated();
        changed = true;
    }
    s.showcase = Some(playback);
    changed
}

pub fn controls(s: &State, k: &Kit) -> El {
    if s.workspace.session_name.is_some() {
        return el();
    }
    let Some(playback) = &s.showcase else {
        return el();
    };
    row().gap(12.0).center_y()
        .child(label(playback.items.iter().find(|i| Some(i.id) == s.selected).map_or("14 examples", |i| i.title.as_str()), 12.0, k.c.text_muted).max_w(180.0).truncate())
        .child(Button::new(if playback.playing { "Pause tour" } else { "Play tour" })
            .icon(if playback.playing { &icons::PAUSE } else { &icons::PLAY })
            .variant(ButtonVariant::Ghost)
            .help("Cycle through example layouts every 3 seconds. Selecting a request stops the tour.")
            .on_press_tx(|s: &mut State, tx| {
                let playing = s.showcase.as_ref().is_some_and(|p| p.playing);
                if !playing {
                    s.workspace.clear_before = 0;
                    s.reset_filters();
                    s.request_text.search.focused = false;
                    s.response_text.search.focused = false;
                    tx.blur();
                }
                if let Some(p) = &mut s.showcase { p.playing = !playing; p.next_at = None; }
                tx.wake_after(Duration::ZERO);
            }).build(k).key("showcase-tour"))
}
