use crate::{State, feed};
use jev_protocol::{Exchange, PROTOCOL_VERSION, Snapshot, Summary};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

const MAX_FILE: u64 = 64 * 1024 * 1024;
#[cfg(test)]
#[path = "session_live_tests.rs"]
mod live_tests;
#[derive(Serialize, Deserialize)]
pub struct Session {
    pub format: String,
    pub version: u32,
    pub snapshot: Snapshot,
    pub exchanges: BTreeMap<u64, Exchange>,
    pub starred: BTreeSet<u64>,
    pub missing: Vec<u64>,
    #[serde(default)]
    pub last_received: bool,
}
pub enum ResultValue {
    Saved(PathBuf, usize, bool),
    Loaded(Box<Session>, PathBuf),
    Cancelled,
}
#[derive(Default)]
pub struct Job {
    pub result: Mutex<Option<Result<ResultValue, String>>>,
    pub done: AtomicUsize,
    pub total: AtomicUsize,
    pub cancel: AtomicBool,
}
pub fn read(path: &Path) -> Result<Session, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open session: {e}"))?;
    let mut bytes = vec![];
    file.take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("Session exceeds 64 MB.".into());
    }
    let session: Session =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid session: {e}"))?;
    validate(&session)?;
    Ok(session)
}
pub fn validate(s: &Session) -> Result<(), String> {
    if s.format != "jev-inspector-session"
        || s.version != 1
        || s.snapshot.version != PROTOCOL_VERSION
    {
        return Err("Unsupported session format or version.".into());
    }
    if s.snapshot.requests.len() > 200
        || s.exchanges.len() > 200
        || s.starred.len() > 200
        || s.missing.len() > 200
    {
        return Err("A session can contain at most 200 exchanges.".into());
    }
    let ids: BTreeSet<_> = s.snapshot.requests.iter().map(|r| r.id).collect();
    if ids.len() != s.snapshot.requests.len()
        || ids.contains(&0)
        || s.exchanges
            .iter()
            .any(|(id, e)| *id != e.summary.id || !ids.contains(id))
        || s.starred
            .iter()
            .chain(&s.missing)
            .any(|id| !ids.contains(id))
    {
        return Err("Session contains duplicate or inconsistent exchange IDs.".into());
    }
    for r in s
        .snapshot
        .requests
        .iter()
        .chain(s.exchanges.values().map(|e| &e.summary))
    {
        if r.duration_ms.is_some_and(|v| !v.is_finite() || v < 0.0)
            || r.upstream_headers_ms
                .is_some_and(|v| !v.is_finite() || v < 0.0)
        {
            return Err("Session contains invalid timing data.".into());
        }
    }
    for e in s.exchanges.values() {
        for c in [&e.request, &e.response] {
            if serde_json::to_vec(c).map_err(|e| e.to_string())?.len() > 512 * 1024 {
                return Err("A session capture exceeds 512 KB.".into());
            }
        }
    }
    Ok(())
}
pub fn csv(rows: &[Summary]) -> String {
    fn field(s: &str) -> String {
        // Spreadsheet imports must not turn untrusted model/path values into formulas.
        let protect = s.trim_start().starts_with(['=', '+', '-', '@', '\t', '\r']);
        format!(
            "\"{}{}\"",
            if protect { "'" } else { "" },
            s.replace('"', "\"\"")
        )
    }
    let mut out = "id,started_at_ms,method,path,model,status,outcome,duration_ms,request_bytes,response_bytes,request_id\r\n".to_string();
    for r in rows {
        let fields = [
            r.id.to_string(),
            r.started_at_ms.to_string(),
            r.method.clone(),
            r.path.clone(),
            r.model.clone().unwrap_or_default(),
            r.status.map(|n| n.to_string()).unwrap_or_default(),
            format!("{:?}", r.outcome),
            r.duration_ms.map(|n| n.to_string()).unwrap_or_default(),
            r.request_bytes.to_string(),
            r.response_bytes.to_string(),
            r.request_id.clone().unwrap_or_default(),
        ];
        out.push_str(
            &fields
                .iter()
                .map(|s| field(s))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out
}
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > MAX_FILE {
        return Err("Export exceeds 64 MB.".into());
    }
    // Stage beside the destination. A failed serialization/write never destroys an old export.
    let temporary = path.with_file_name(format!(
        ".jev-export-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| {
        let mut f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("Cannot save {}: {e}", path.display()))
}
pub fn save_dialog(s: &mut State, tx: &mut rune::Tx, kind: u8) {
    crate::showcase::stop(s);
    if s.workspace.job.is_some() {
        return;
    }
    let (title, name, ext) = match kind {
        1 => (
            "Export visible request summaries",
            "jev-requests.csv",
            "csv",
        ),
        2 => ("Export selected exchange", "jev-exchange.json", "json"),
        _ => (
            "Save visible requests as a session",
            "jev-session.json",
            "json",
        ),
    };
    tx.save_file(
        rune::FileDialogOptions {
            title: title.into(),
            file_name: Some(name.into()),
            ..Default::default()
        }
        .filter(title, &[ext]),
        move |s: &mut State, result, _| match result {
            Ok(Some(path)) => start_save(s, path, kind),
            Ok(None) => {}
            Err(e) => s.workspace.notice = Some((e.to_string(), true)),
        },
    );
}
pub fn open_dialog(s: &mut State, tx: &mut rune::Tx) {
    crate::showcase::stop(s);
    if s.workspace.job.is_some() {
        return;
    }
    tx.open_file(
        rune::FileDialogOptions {
            title: "Open Jev session".into(),
            ..Default::default()
        }
        .filter("Jev session", &["json"]),
        |s: &mut State, result, _| match result {
            Ok(Some(path)) => {
                let job = Arc::new(Job::default());
                s.workspace.job = Some(job.clone());
                let waker = s.workspace.waker.clone();
                std::thread::spawn(move || {
                    let result = read(&path).map(|s| {
                        if job.cancel.load(Ordering::Relaxed) {
                            ResultValue::Cancelled
                        } else {
                            ResultValue::Loaded(Box::new(s), path)
                        }
                    });
                    *job.result.lock().unwrap() = Some(result);
                    if let Some(wake) = waker {
                        wake();
                    }
                });
            }
            Ok(None) => {}
            Err(e) => s.workspace.notice = Some((e.to_string(), true)),
        },
    );
}
fn start_save(s: &mut State, path: PathBuf, kind: u8) {
    let mut snapshot = s.snapshot.clone();
    snapshot.requests = s.visible.clone();
    let mut exchanges = s.workspace.cache.clone();
    exchanges.retain(|id, _| snapshot.requests.iter().any(|r| r.id == *id));
    let starred = s
        .workspace
        .starred
        .iter()
        .filter(|id| snapshot.requests.iter().any(|r| r.id == **id))
        .copied()
        .collect();
    let offline = s.demo || s.offline_fixture || s.workspace.session_name.is_some();
    let disconnected = s.connection_error.is_some();
    let mut last_received = s.workspace.last_received || (!offline && disconnected);
    if s.demo && s.showcase.is_none() && s.workspace.session_name.is_none() {
        // Legacy fixtures generate uncached captures on demand. Preserve captured
        // values already in the cache, and never use that generator for the catalog.
        for r in &snapshot.requests {
            exchanges
                .entry(r.id)
                .or_insert_with(|| crate::demo::exchange(r.id));
        }
    }
    let detail = s.detail.clone();
    let connection_path = s.connection_file.clone();
    let job = Arc::new(Job::default());
    job.total.store(snapshot.requests.len(), Ordering::Relaxed);
    s.workspace.job = Some(job.clone());
    let waker = s.workspace.waker.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<ResultValue, String> {
            let mut missing = vec![];
            let bytes = match kind {
                1 => csv(&snapshot.requests).into_bytes(),
                2 => serde_json::to_vec_pretty(&detail.ok_or("No exchange selected.")?)
                    .map_err(|e| e.to_string())?,
                _ => {
                    if !offline && !disconnected {
                        enum RefreshFailure {
                            Unavailable,
                            Restarted,
                            Cancelled,
                        }
                        let cached = exchanges.clone();
                        let refresh = (|| -> Result<(), RefreshFailure> {
                            let connection = feed::connection(&connection_path)
                                .map_err(|_| RefreshFailure::Unavailable)?;
                            let client = reqwest::blocking::Client::builder()
                                .no_proxy()
                                .redirect(reqwest::redirect::Policy::none())
                                .retry(reqwest::retry::never())
                                .timeout(Duration::from_secs(2))
                                .build()
                                .map_err(|_| RefreshFailure::Unavailable)?;
                            let current: Snapshot =
                                feed::get(&client, &connection, "/api/requests")
                                    .map_err(|_| RefreshFailure::Unavailable)?;
                            if current.instance_id != snapshot.instance_id {
                                return Err(RefreshFailure::Restarted);
                            }
                            for (i, row) in snapshot.requests.iter().enumerate() {
                                if job.cancel.load(Ordering::Relaxed) {
                                    return Err(RefreshFailure::Cancelled);
                                }
                                match feed::get::<Exchange>(
                                    &client,
                                    &connection,
                                    &format!("/api/requests/{}", row.id),
                                ) {
                                    Ok(exchange) => {
                                        exchanges.insert(row.id, exchange);
                                    }
                                    Err(_) => {
                                        // Distinguish one evicted body from an outage before
                                        // spending another timeout on every retained exchange.
                                        let current: Snapshot =
                                            feed::get(&client, &connection, "/api/requests")
                                                .map_err(|_| RefreshFailure::Unavailable)?;
                                        if current.instance_id != snapshot.instance_id {
                                            return Err(RefreshFailure::Restarted);
                                        }
                                    }
                                }
                                job.done.store(i + 1, Ordering::Relaxed);
                                if let Some(wake) = &waker {
                                    wake();
                                }
                            }
                            let current: Snapshot =
                                feed::get(&client, &connection, "/api/requests")
                                    .map_err(|_| RefreshFailure::Unavailable)?;
                            if current.instance_id != snapshot.instance_id {
                                return Err(RefreshFailure::Restarted);
                            }
                            Ok(())
                        })();
                        match refresh {
                            Ok(()) => {}
                            Err(RefreshFailure::Cancelled) => return Ok(ResultValue::Cancelled),
                            Err(RefreshFailure::Restarted) => {
                                return Err(
                                    "Relay restarted during export. No session was written.".into(),
                                );
                            }
                            Err(RefreshFailure::Unavailable) => {
                                exchanges = cached;
                                last_received = true;
                            }
                        }
                    }
                    missing = snapshot
                        .requests
                        .iter()
                        .filter(|r| !exchanges.contains_key(&r.id))
                        .map(|r| r.id)
                        .collect();
                    // Pair each saved summary with the version of its saved body.
                    for r in &mut snapshot.requests {
                        if let Some(e) = exchanges.get(&r.id) {
                            *r = e.summary.clone();
                        }
                    }
                    let session = Session {
                        format: "jev-inspector-session".into(),
                        version: 1,
                        snapshot,
                        exchanges,
                        starred,
                        missing: missing.clone(),
                        last_received,
                    };
                    validate(&session)?;
                    serde_json::to_vec_pretty(&session).map_err(|e| e.to_string())?
                }
            };
            if job.cancel.load(Ordering::Relaxed) {
                return Ok(ResultValue::Cancelled);
            }
            write(&path, &bytes)?;
            Ok(ResultValue::Saved(
                path,
                missing.len(),
                kind == 0 && last_received,
            ))
        })();
        *job.result.lock().unwrap() = Some(result);
        if let Some(wake) = waker {
            wake();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_round_trip_and_validation() {
        let mut state = State::default();
        crate::demo::populate(&mut state);
        let mut s = Session {
            format: "jev-inspector-session".into(),
            version: 1,
            snapshot: state.snapshot,
            exchanges: [(12, crate::demo::exchange(12))].into(),
            starred: [12].into(),
            missing: vec![],
            last_received: false,
        };
        let bytes = serde_json::to_vec(&s).unwrap();
        validate(&serde_json::from_slice(&bytes).unwrap()).unwrap();
        s.snapshot.requests.push(s.snapshot.requests[0].clone());
        assert!(validate(&s).is_err());
        s.snapshot.requests.pop();
        s.version = 2;
        assert!(validate(&s).is_err());
    }
    #[test]
    fn csv_escapes_quotes_newlines_and_formulas() {
        let mut r = crate::demo::exchange(1).summary;
        r.model = Some("=SUM(1,2)".into());
        r.path = "a\"b\nc".into();
        let csv = csv(&[r]);
        assert!(csv.contains("\"'=SUM(1,2)\""));
        assert!(csv.contains("\"a\"\"b\nc\""));
    }
}
