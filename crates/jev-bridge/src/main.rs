use clap::Parser;
use jev_protocol::{Connection, PROTOCOL_VERSION};
use std::{net::SocketAddr, path::PathBuf, time::Duration};

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
    let token = jev_bridge::new_token();
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
    jev_bridge::write_connection(&args.connection_file, &connection)?;
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
    let result = bridge
        .serve(relay, inspector, async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    jev_bridge::remove_connection(&args.connection_file, &connection.token);
    result?;
    Ok(())
}
