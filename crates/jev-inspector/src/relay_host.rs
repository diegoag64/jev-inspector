//! The relay the desktop app runs for itself. In app mode the inspector hosts
//! `jev-bridge` on a background thread, on the port chosen during onboarding,
//! so there is nothing to start in a terminal. Developer builds still connect
//! to a separately started relay through its connection file.
use jev_protocol::{Connection, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use std::{
    io::ErrorKind,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};

pub const DEFAULT_PORT: u16 = 8787;
pub const UPSTREAM: &str = "https://api.typesafe.ai";
/// Ports below this need administrator rights on most systems.
pub const MIN_PORT: u16 = 1024;
const HISTORY: usize = 100;
const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub port: u16,
    pub capture_bodies: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            capture_bodies: true,
        }
    }
}

pub fn relay_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// Read a port the user typed. Blank, non-numeric and privileged ports are rejected.
pub fn parse_port(text: &str) -> Result<u16, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Enter a port number.".into());
    }
    match text.parse::<u32>() {
        Ok(port) if (u32::from(MIN_PORT)..=65535).contains(&port) => Ok(port as u16),
        _ => Err(format!("Use a number from {MIN_PORT} to 65535.")),
    }
}

fn bind(port: u16) -> Result<TcpListener, String> {
    TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).map_err(|e| {
        if e.kind() == ErrorKind::AddrInUse {
            format!("Another app is using port {port}.")
        } else {
            format!("Port {port} is unavailable: {e}")
        }
    })
}

/// Whether the relay could listen on `port` right now.
pub fn check(port: u16) -> Result<(), String> {
    bind(port).map(drop)
}

/// The next free port after `port`, for the "Use … instead" suggestion.
pub fn suggest(port: u16) -> Option<u16> {
    (1..=64)
        .filter_map(|step| port.checked_add(step))
        .find(|&p| check(p).is_ok())
}

struct Running {
    settings: Settings,
    connection_file: PathBuf,
    token: String,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        jev_bridge::remove_connection(&self.connection_file, &self.token);
    }
}

/// Owns the hosted relay. Clones share it, so `main` can stop the relay after the
/// window closes even though the UI state has moved into the event loop.
#[derive(Clone)]
pub struct Host {
    pub connection_file: PathBuf,
    running: Arc<Mutex<Option<Running>>>,
}

impl Host {
    pub fn new(connection_file: PathBuf) -> Self {
        Self {
            connection_file,
            running: Arc::default(),
        }
    }

    /// The settings of the relay that is running now, if any.
    pub fn running(&self) -> Option<Settings> {
        self.running
            .lock()
            .expect("relay host")
            .as_ref()
            .map(|r| r.settings)
    }

    /// Stop any running relay and start one with `settings`. On failure the previous
    /// relay is restarted so a typo never leaves the app without capture.
    pub fn start(&self, settings: Settings) -> Result<(), String> {
        let mut slot = self.running.lock().expect("relay host");
        let previous = slot.take().map(|r| r.settings);
        match start(settings, &self.connection_file) {
            Ok(running) => {
                *slot = Some(running);
                Ok(())
            }
            Err(error) => {
                if let Some(previous) = previous {
                    *slot = start(previous, &self.connection_file).ok();
                }
                Err(error)
            }
        }
    }

    pub fn stop(&self) {
        self.running.lock().expect("relay host").take();
    }
}

fn start(settings: Settings, connection_file: &Path) -> Result<Running, String> {
    let relay = bind(settings.port)?;
    let inspector = bind(0)?;
    let relay_addr = relay.local_addr().map_err(|e| e.to_string())?;
    let inspector_addr = inspector.local_addr().map_err(|e| e.to_string())?;
    for listener in [&relay, &inspector] {
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    }
    let token = jev_bridge::new_token();
    let bridge = jev_bridge::Bridge::new(
        UPSTREAM,
        relay_addr,
        inspector_addr,
        token.clone(),
        settings.capture_bodies,
        HISTORY,
        TIMEOUT,
    )?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("jev-relay")
        .enable_all()
        .build()
        .map_err(|e| format!("Cannot start the relay: {e}"))?;
    let (relay, inspector) = {
        let _guard = runtime.enter();
        (
            tokio::net::TcpListener::from_std(relay).map_err(|e| e.to_string())?,
            tokio::net::TcpListener::from_std(inspector).map_err(|e| e.to_string())?,
        )
    };
    let connection = Connection {
        version: PROTOCOL_VERSION,
        inspector_url: format!("http://{inspector_addr}"),
        token: token.clone(),
    };
    jev_bridge::write_connection(connection_file, &connection)
        .map_err(|e| format!("Cannot write {}: {e}", connection_file.display()))?;
    let (shutdown, stopped) = tokio::sync::oneshot::channel::<()>();
    let thread = std::thread::Builder::new()
        .name("jev-relay".into())
        .spawn(move || {
            let result = runtime.block_on(bridge.serve(relay, inspector, async {
                let _ = stopped.await;
            }));
            if let Err(error) = result {
                eprintln!("Relay stopped: {error}");
            }
        })
        .map_err(|e| format!("Cannot start the relay: {e}"))?;
    Ok(Running {
        settings,
        connection_file: connection_file.to_owned(),
        token,
        shutdown: Some(shutdown),
        thread: Some(thread),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_must_be_unprivileged_numbers() {
        assert_eq!(parse_port(" 8790 "), Ok(8790));
        assert!(parse_port("").is_err());
        assert!(parse_port("80").is_err());
        assert!(parse_port("65536").is_err());
        assert!(parse_port("87a7").is_err());
    }

    #[test]
    fn hosted_relay_reports_a_busy_port_and_keeps_the_previous_relay() {
        let dir = std::env::temp_dir().join(format!("jev-host-{}", jev_bridge::new_token()));
        let file = dir.join("connection.json");
        let host = Host::new(file.clone());
        let blocker = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let busy = blocker.local_addr().unwrap().port();
        let free = suggest(busy).expect("a free port");
        drop(TcpListener::bind((Ipv4Addr::LOCALHOST, free)).unwrap());

        let first = Settings {
            port: free,
            capture_bodies: false,
        };
        host.start(first).unwrap();
        assert_eq!(host.running(), Some(first));
        assert!(check(free).is_err(), "the relay owns its port");
        let connection = crate::feed::connection(&file).unwrap();
        assert!(connection.inspector_url.starts_with("http://127.0.0.1:"));

        let error = host
            .start(Settings {
                port: busy,
                capture_bodies: true,
            })
            .unwrap_err();
        assert!(error.contains(&busy.to_string()), "{error}");
        assert_eq!(
            host.running(),
            Some(first),
            "a failed change keeps capture running"
        );

        host.stop();
        assert_eq!(host.running(), None);
        assert!(!file.exists(), "stopping removes the connection file");
        assert!(check(free).is_ok(), "stopping frees the port");
        let _ = std::fs::remove_dir_all(dir);
    }
}
