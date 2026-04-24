//! iRacing Pitwall — Bridge
//!
//! Tokio runtime, WebSocket server, iRacing SDK reader.
//!
//! Entry point wires up logging, spawns the SDK reader task (Windows only),
//! and starts the WebSocket server that pushes snapshots to the dashboard.

#![cfg_attr(not(windows), allow(dead_code))]

mod config;
mod error;
mod iracing_sdk;
mod telemetry;
mod ws;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let cfg = config::Config::from_env();
    log::info!(
        "iracing-pitwall bridge starting on ws://127.0.0.1:{}",
        cfg.ws_port
    );

    #[cfg(not(windows))]
    {
        log::warn!("Non-Windows build: iRacing SDK reader disabled (stub only).");
    }

    // TODO: spawn reader task, ws server, wire broadcast channel.
    todo!("wire up reader + ws server");
}

fn init_logging() {
    use env_logger::Env;
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
}
