//! iRacing Pitwall — Bridge
//!
//! Tokio runtime, HTTP+WebSocket server, iRacing SDK reader.

#![allow(dead_code)]

mod config;
mod error;
mod iracing_sdk;
mod telemetry;
mod ws;

use std::path::PathBuf;

use anyhow::Result;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let cfg = config::Config::from_env();
    log::info!(
        "iracing-pitwall bridge v{} starting on http://127.0.0.1:{}",
        env!("CARGO_PKG_VERSION"),
        cfg.ws_port
    );

    let (tel_tx, tel_rx) = watch::channel(None::<telemetry::TelemetrySnapshot>);
    let (std_tx, std_rx) = watch::channel(None::<telemetry::StandingsSnapshot>);
    let (si_tx, si_rx) = watch::channel(None::<iracing_sdk::types::SessionInfoYaml>);

    let state = ws::BridgeState {
        telemetry: tel_rx,
        standings: std_rx,
        session_info: si_rx,
    };

    #[cfg(windows)]
    tokio::task::spawn_blocking(move || {
        if let Err(e) = sdk_loop(tel_tx, std_tx, si_tx) {
            log::error!("sdk_loop terminated: {e}");
        }
    });

    #[cfg(not(windows))]
    {
        log::warn!("Non-Windows build: iRacing SDK reader disabled.");
        drop((tel_tx, std_tx, si_tx));
    }

    let ws_task = tokio::spawn(ws::serve(cfg.ws_port, state));

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log::info!("ctrl-c received, shutting down");
        }
        r = ws_task => {
            match r {
                Ok(Err(e)) => log::error!("ws server error: {e}"),
                Err(e) => log::error!("ws task panicked: {e}"),
                Ok(Ok(())) => {}
            }
        }
    }
    Ok(())
}

/// Blocking SDK loop — runs in spawn_blocking, owns the IRacingClient for its lifetime.
/// Feeds three watch channels: telemetry at ~60 Hz, standings + session-info at ~4 Hz.
#[cfg(windows)]
fn sdk_loop(
    tel_tx: watch::Sender<Option<telemetry::TelemetrySnapshot>>,
    std_tx: watch::Sender<Option<telemetry::StandingsSnapshot>>,
    si_tx: watch::Sender<Option<iracing_sdk::types::SessionInfoYaml>>,
) -> Result<()> {
    use iracing_sdk::yaml::decode_and_parse;

    let mut client = iracing_sdk::IRacingClient::connect()?;
    client.parse_var_index()?;

    let mut last_si_update: i32 = -1;
    let mut yaml_cache: Option<iracing_sdk::types::SessionInfoYaml> = None;
    let mut frame: u64 = 0;

    loop {
        client.wait_for_frame()?;

        let _ = tel_tx.send(Some(telemetry::TelemetrySnapshot::build(&client)?));

        // Every 15 frames ≈ 250 ms at 60 Hz → 4 Hz standings + session-info
        if frame % 15 == 0 {
            let cur = client.session_info_update();
            if cur != last_si_update || yaml_cache.is_none() {
                let yaml = decode_and_parse(client.session_info_bytes())?;
                let _ = si_tx.send(Some(yaml.clone()));
                yaml_cache = Some(yaml);
                last_si_update = cur;
            }
            if let Some(ref y) = yaml_cache {
                let _ = std_tx.send(Some(telemetry::StandingsSnapshot::build(&client, y)?));
            }
        }

        frame = frame.wrapping_add(1);
    }
}

fn log_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("bridge.log")))
        .unwrap_or_else(|| PathBuf::from("bridge.log"))
}

fn init_logging() {
    use simplelog::{
        ColorChoice, CombinedLogger, ConfigBuilder, LevelFilter, TermLogger, TerminalMode,
        WriteLogger,
    };

    let level = std::env::var("BRIDGE_LOG")
        .ok()
        .and_then(|v| match v.to_ascii_lowercase().as_str() {
            "trace" => Some(LevelFilter::Trace),
            "debug" => Some(LevelFilter::Debug),
            "info" => Some(LevelFilter::Info),
            "warn" => Some(LevelFilter::Warn),
            "error" => Some(LevelFilter::Error),
            _ => None,
        })
        .unwrap_or(LevelFilter::Info);

    let cfg = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .set_target_level(LevelFilter::Error)
        .build();

    let path = log_path();
    let term = TermLogger::new(level, cfg.clone(), TerminalMode::Mixed, ColorChoice::Auto);

    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![term];
    let mut file_err: Option<String> = None;
    match std::fs::File::create(&path) {
        Ok(file) => loggers.push(WriteLogger::new(level, cfg, file)),
        Err(e) => file_err = Some(format!("failed to open log file {}: {e}", path.display())),
    }
    let _ = CombinedLogger::init(loggers);
    if let Some(msg) = file_err {
        log::warn!("{msg}");
    } else {
        log::info!("log file: {}", path.display());
    }
}
