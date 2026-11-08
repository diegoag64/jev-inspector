use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, mpsc},
};

use jev_protocol::{Exchange, PROTOCOL_VERSION, Snapshot, Summary};

use crate::capture::{RawCapture, inspect};

pub struct Work {
    pub id: u64,
    pub request: RawCapture,
    pub response: RawCapture,
}

pub struct Store {
    pub snapshot: Snapshot,
    entries: VecDeque<Exchange>,
}

impl Store {
    pub fn new(
        relay_url: String,
        upstream: String,
        capture_bodies: bool,
        history_limit: usize,
    ) -> Self {
        Self {
            snapshot: Snapshot {
                version: PROTOCOL_VERSION,
                instance_id: uuid::Uuid::new_v4().to_string(),
                relay_url,
                upstream,
                capture_bodies,
                history_limit,
                ..Snapshot::default()
            },
            entries: VecDeque::new(),
        }
    }
    pub fn start(&mut self, mut summary: Summary) -> u64 {
        self.snapshot.total_requests += 1;
        summary.id = self.snapshot.total_requests;
        let id = summary.id;
        if self.entries.len() == self.snapshot.history_limit {
            self.entries.pop_back();
            self.snapshot.evicted += 1;
        }
        self.entries.push_front(Exchange {
            summary,
            ..Exchange::default()
        });
        self.snapshot.revision += 1;
        id
    }
    pub fn update(&mut self, id: u64, change: impl FnOnce(&mut Exchange)) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.summary.id == id) {
            change(entry);
            self.snapshot.revision += 1;
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            requests: self.entries.iter().map(|e| e.summary.clone()).collect(),
            ..self.snapshot.clone()
        }
    }
    pub fn get(&self, id: u64) -> Option<Exchange> {
        self.entries.iter().find(|e| e.summary.id == id).cloned()
    }
}

pub fn worker(store: Arc<Mutex<Store>>) -> mpsc::SyncSender<Work> {
    let (sender, receiver) = mpsc::sync_channel::<Work>(32);
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("capture worker runtime");
        while let Ok(work) = receiver.recv() {
            let request = runtime.block_on(inspect(work.request));
            let response = runtime.block_on(inspect(work.response));
            let model = response
                .json
                .as_ref()
                .and_then(|v| v.get("model"))
                .or_else(|| request.json.as_ref().and_then(|v| v.get("model")))
                .and_then(|v| v.as_str())
                .map(|s| s.chars().take(120).collect());
            store
                .lock()
                .expect("capture store")
                .update(work.id, |entry| {
                    entry.request = request;
                    entry.summary.usage = response.token_usage();
                    entry.response = response;
                    entry.summary.model = model;
                });
        }
    });
    sender
}
