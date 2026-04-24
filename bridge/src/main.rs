//! iRacing Pitwall — Bridge
//!
//! Tokio runtime, WebSocket server, iRacing SDK reader.
//!
//! Entry point wires up logging, spawns the SDK reader task (Windows only),
//! and starts the WebSocket server that pushes snapshots to the dashboard.

#![allow(dead_code)]

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

    // Try to connect to iRacing
    match iracing_sdk::IRacingClient::connect() {
        Ok(client) => {
            let header = client.header();
            log::info!("Connected to iRacing SDK");
            log::info!(
                "Header: ver={}, tick_rate={}, num_vars={}, buf_len={}, connected={}",
                header.ver,
                header.tick_rate,
                header.num_vars,
                header.buf_len,
                header.is_connected()
            );

            // Acceptance test: lookup known telemetry variables
            let var_index = client.var_index();
            log::info!("var_index contains {} entries", var_index.len());
            for name in ["Speed", "Throttle", "SessionTime"] {
                match var_index.get(name) {
                    Some(v) => {
                        log::info!(
                            " {}: type={:?} offset={} count={} unit={:?}",
                            name,
                            v.var_type,
                            v.offset,
                            v.count,
                            v.unit
                        );
                    }
                    None => {
                        log::warn!(" {}: NOT FOUND", name);
                    }
                }
            }

            // Client will be dropped here when it goes out of scope
            // For now we just log and exit normally
        }
        Err(e) => {
            #[cfg(not(windows))]
            {
                log::warn!(
                    "Failed to connect to iRacing SDK (expected on non-Windows): {}",
                    e
                );
                // Normal exit on non-Windows during dev
                return Ok(());
            }
            #[cfg(windows)]
            {
                log::error!("Failed to connect to iRacing SDK: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}

fn init_logging() {
    use env_logger::Env;
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
}
