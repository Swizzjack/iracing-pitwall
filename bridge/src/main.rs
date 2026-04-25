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

    // Standings snapshot (once, from first frame)
    client.wait_for_frame()?;
    let yaml_info = iracing_sdk::yaml::decode_and_parse(client.session_info_bytes());
    if let Ok(ref yaml) = yaml_info {
        match telemetry::StandingsSnapshot::build(&client, yaml) {
            Ok(s) => log::info!(
                "Standings: session={} type={} entries={}",
                s.session_num,
                s.session_type,
                s.entries.len()
            ),
            Err(e) => log::warn!("StandingsSnapshot failed: {e}"),
        }
    }

    for _ in 0..10 {
        client.wait_for_frame()?;
        let snap = telemetry::TelemetrySnapshot::build(&client)?;
        log::info!(
            "Speed={:.1} m/s  Gear={}  Lap={}  Fuel={:.1}%  Pos={}  Pit={}",
            snap.speed_ms,
            snap.gear,
            snap.lap,
            snap.fuel_level_pct * 100.0,
            snap.player_position,
            snap.on_pit_road,
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
