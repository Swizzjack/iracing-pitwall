//! iRacing Pitwall — Bridge
//!
//! Tokio runtime, WebSocket server, iRacing SDK reader.

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
        log::warn!("Non-Windows build: iRacing SDK reader disabled.");
        return Ok(());
    }

    #[cfg(windows)]
    run_demo_loop()
}

/// Task 3+4+5 acceptance: session info once, then 10 frames of telemetry.
#[cfg(windows)]
fn run_demo_loop() -> Result<()> {
    let mut client = iracing_sdk::IRacingClient::connect()?;
    client.parse_var_index()?;

    client.wait_for_frame()?;
    match iracing_sdk::yaml::decode_and_parse(client.session_info_bytes()) {
        Ok(info) => log::info!(
            "SessionInfo: track={} sessions={} drivers={} update={}",
            info.weekend_info.track_name,
            info.session_info.sessions.len(),
            info.driver_info.drivers.len(),
            client.session_info_update(),
        ),
        Err(e) => log::warn!("SessionInfo parse failed: {e}"),
    }

    for _ in 0..10 {
        client.wait_for_frame()?;
        log::info!(
            "Speed={:.2} m/s  Throttle={:.0}%  RPM={:.0}  Gear={}  SessionTime={:.3}",
            client.get_f32("Speed")?,
            client.get_f32("Throttle")? * 100.0,
            client.get_f32("RPM")?,
            client.get_i32("Gear")?,
            client.get_f64("SessionTime")?,
        );
    }
    Ok(())
}

fn init_logging() {
    use env_logger::Env;
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
}
