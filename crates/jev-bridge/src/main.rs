use clap::Parser;
use jev_protocol::{Connection, PROTOCOL_VERSION};
use std::{
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Parser)]
#[command(
    version,
    about = "Local TypeSafe relay with bounded, in-memory inspection. No extra retries."
)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:8787")]
    listen: SocketAddr,
    #[arg(long, default_value = "127.0.0.1:8788")]
    inspector: SocketAddr,
    #[arg(long, default_value = "https://api.typesafe.ai")]
    upstream: String,
    /// Capture JSON inspection copies. Arbitrary sensitive prose is not automatically redacted.
    #[arg(long)]
    capture_bodies: bool,
    #[arg(long, default_value_t = 100)]
    history: usize,
    #[arg(long, default_value_t = 120)]
    timeout_seconds: u64,
    #[arg(long, default_value = ".jev/connection.json")]
    connection_file: PathBuf,
}

fn write_connection(
    path: &Path,
    connection: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".connection-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = options.open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(connection)?)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if !args.listen.ip().is_loopback() || !args.inspector.ip().is_loopback() {
        return Err("Only loopback listeners are supported.".into());
    }
    let relay = tokio::net::TcpListener::bind(args.listen).await?;
    let inspector = tokio::net::TcpListener::bind(args.inspector).await?;
    let relay_addr = relay.local_addr()?;
    let inspector_addr = inspector.local_addr()?;
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let bridge = jev_bridge::Bridge::new(
        &args.upstream,
        relay_addr,
        inspector_addr,
        token.clone(),
        args.capture_bodies,
        args.history,
        Duration::from_secs(args.timeout_seconds),
    )?;
    let connection = Connection {
        version: PROTOCOL_VERSION,
        inspector_url: format!("http://{inspector_addr}"),
        token,
    };
    write_connection(&args.connection_file, &connection)?;
    println!("Jev bridge listening at http://{relay_addr}");
    println!("Inspector connection: {}", args.connection_file.display());
    println!(
        "Body capture: {}. History is in memory and clears when the relay stops.",
        if args.capture_bodies {
            "enabled (best-effort field redaction)"
        } else {
            "disabled"
        }
    );
    println!("Closing the inspector does not stop this process. Ctrl+C stops the relay.");
    let (shutdown, receiver) = tokio::sync::watch::channel(false);
    let mut receiver2 = receiver.clone();
    let mut receiver = receiver;
    let servers = async {
        tokio::try_join!(
            axum::serve(relay, bridge.relay_router()).with_graceful_shutdown(async move {
                let _ = receiver.changed().await;
            }),
            axum::serve(inspector, bridge.inspector_router()).with_graceful_shutdown(async move {
                let _ = receiver2.changed().await;
            })
        )
    };
    tokio::pin!(servers);
    let result = tokio::select! {
        result = &mut servers => result.map(|_| ()),
        result = tokio::signal::ctrl_c() => {
            result?;
            let _ = shutdown.send(true);
            let _ = tokio::time::timeout(Duration::from_secs(5), &mut servers).await;
            Ok(())
        }
    };
    // Do not remove a connection file another instance has since replaced.
    if std::fs::read(&args.connection_file)
        .ok()
        .and_then(|b| serde_json::from_slice::<Connection>(&b).ok())
        .is_some_and(|current| current.token == connection.token)
    {
        let _ = std::fs::remove_file(&args.connection_file);
    }
    result?;
    Ok(())
}
