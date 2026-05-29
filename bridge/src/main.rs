//! iRacing Pitwall — Bridge
//!
//! Tokio runtime, HTTP+WebSocket server, iRacing SDK reader.

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#![allow(dead_code)]

mod config;
mod error;
mod iracing_sdk;
mod telemetry;
mod update;
mod ws;

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
#[cfg(windows)]
use anyhow::Context;
use tokio::sync::{oneshot, watch};

/// Prüft per TCP-Connect ob bereits eine Bridge-Instanz auf diesem Port läuft.
/// Schneller als port-binding und funktioniert ohne Windows-spezifische APIs.
fn is_already_running(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(300),
    )
    .is_ok()
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    let cfg = config::Config::from_env();
    log::info!(
        "iracing-pitwall bridge v{} starting on http://127.0.0.1:{}",
        env!("CARGO_PKG_VERSION"),
        cfg.ws_port
    );

    // Läuft bereits eine Instanz? → Browser öffnen und beenden.
    if is_already_running(cfg.ws_port) {
        log::info!(
            "bridge already running on port {} — opening browser and exiting",
            cfg.ws_port
        );
        let url = format!("http://127.0.0.1:{}/", cfg.ws_port);
        if let Err(e) = webbrowser::open(&url) {
            log::warn!("failed to open browser: {e}");
        }
        return Ok(());
    }

    let (tel_tx, tel_rx) = watch::channel(None::<telemetry::TelemetrySnapshot>);
    let (std_tx, std_rx) = watch::channel(None::<telemetry::StandingsSnapshot>);
    let (si_tx, si_rx) = watch::channel(None::<iracing_sdk::types::SessionInfoYaml>);
    let (tm_tx, tm_rx) = watch::channel(None::<telemetry::TrackMapSnapshot>);

    // Update check — runs in a background thread so it never delays startup.
    // upd_tx is kept alive via pending() so the watch channel is never closed;
    // a closed sender causes changed() to return Err and would disconnect all WS clients.
    // Set BRIDGE_VERSION_OVERRIDE=0.1.0 to test the "update available" UI.
    let (upd_tx, upd_rx) = watch::channel(None::<update::UpdateInfo>);
    let current_version = std::env::var("BRIDGE_VERSION_OVERRIDE")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    tokio::spawn(async move {
        let info = tokio::task::spawn_blocking(move || {
            update::check_for_update(&current_version)
        })
        .await
        .ok()
        .flatten();
        if let Some(i) = info {
            log::info!("update available: v{}", i.latest_version);
            let _ = upd_tx.send(Some(i));
        }
        // Keep upd_tx alive so the watch channel stays open for all WS clients.
        std::future::pending::<()>().await;
    });

    let lan_url = detect_lan_ip().map(|ip| format!("http://{}:{}/", ip, cfg.ws_port));
    if let Some(ref url) = lan_url {
        log::info!("LAN access: {url}");
    }

    let (clients, count_rx) = ws::ClientTracker::new();
    let state = ws::BridgeState {
        telemetry: tel_rx,
        standings: std_rx,
        session_info: si_rx,
        track_map: tm_rx,
        update: upd_rx,
        clients,
        lan_url,
    };

    #[cfg(windows)]
    tokio::task::spawn_blocking(move || {
        if let Err(e) = sdk_loop(tel_tx, std_tx, si_tx, tm_tx) {
            log::error!("sdk_loop terminated: {e}");
        }
    });

    #[cfg(not(windows))]
    {
        log::warn!("Non-Windows build: iRacing SDK reader disabled.");
        drop((tel_tx, std_tx, si_tx, tm_tx));
    }

    let (addr, listener) = match ws::bind(cfg.ws_port).await {
        Ok(result) => result,
        Err(crate::error::BridgeError::Io(ref e))
            if e.kind() == std::io::ErrorKind::AddrInUse =>
        {
            log::info!(
                "bridge already running on port {} — opening browser and exiting",
                cfg.ws_port
            );
            let url = format!("http://127.0.0.1:{}/", cfg.ws_port);
            if let Err(e) = webbrowser::open(&url) {
                log::warn!("failed to open browser at {url}: {e}");
            }
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };
    if std::env::var("BRIDGE_NO_BROWSER").is_err() {
        let url = format!("http://127.0.0.1:{}/", addr.port());
        if let Err(e) = webbrowser::open(&url) {
            log::warn!("failed to open browser at {url}: {e}");
        }
    }

    let (sd_tx, sd_rx) = oneshot::channel::<()>();
    let lifecycle_cfg = ws::LifecycleConfig {
        keep_alive: cfg.keep_alive,
        grace: Duration::from_secs(cfg.shutdown_grace_sec),
        startup_grace: Duration::from_secs(cfg.startup_grace_sec),
    };
    tokio::spawn(ws::lifecycle::run_watcher(count_rx, lifecycle_cfg, sd_tx));

    let ws_task = tokio::spawn(ws::serve(listener, state, sd_rx));

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
    // spawn_blocking(sdk_loop) würde die Runtime beim Drop blockieren (wait_for_frame
    // ist blockierend). process::exit beendet alle Threads sofort und sauber.
    std::process::exit(0);
}

/// Blocking SDK loop — runs in spawn_blocking, owns the IRacingClient for its lifetime.
/// Retries indefinitely until iRacing is running, reconnects after disconnect.
#[cfg(windows)]
fn sdk_loop(
    tel_tx: watch::Sender<Option<telemetry::TelemetrySnapshot>>,
    std_tx: watch::Sender<Option<telemetry::StandingsSnapshot>>,
    si_tx: watch::Sender<Option<iracing_sdk::types::SessionInfoYaml>>,
    tm_tx: watch::Sender<Option<telemetry::TrackMapSnapshot>>,
) -> Result<()> {
    const RETRY_DELAY: Duration = Duration::from_secs(3);

    loop {
        match connect_and_run(&tel_tx, &std_tx, &si_tx, &tm_tx) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if matches!(
                    e.downcast_ref::<crate::error::BridgeError>(),
                    Some(crate::error::BridgeError::SdkNotConnected(_))
                ) {
                    log::info!("iRacing not connected, retrying in {}s", RETRY_DELAY.as_secs());
                } else {
                    log::warn!("connect_and_run failed, retrying in {}s: {e:#}", RETRY_DELAY.as_secs());
                }
                std::thread::sleep(RETRY_DELAY);
            }
        }
    }
}

#[cfg(windows)]
fn connect_and_run(
    tel_tx: &watch::Sender<Option<telemetry::TelemetrySnapshot>>,
    std_tx: &watch::Sender<Option<telemetry::StandingsSnapshot>>,
    si_tx: &watch::Sender<Option<iracing_sdk::types::SessionInfoYaml>>,
    tm_tx: &watch::Sender<Option<telemetry::TrackMapSnapshot>>,
) -> Result<()> {
    use iracing_sdk::yaml::decode_and_parse;
    use iracing_sdk::synthetic_id::SyntheticSubSessionId;

    let mut client = iracing_sdk::IRacingClient::connect().context("iracing connect")?;
    client.parse_var_index().context("parse_var_index")?;
    log::info!("iRacing connected");

    let cache_dir = telemetry::track_recorder::cache_dir_from_exe();
    let mut last_si_update: i32 = -1;
    let mut yaml_cache: Option<iracing_sdk::types::SessionInfoYaml> = None;
    let mut pit_tracker = telemetry::pit_tracker::PitTracker::default();
    let mut sector_tracker = telemetry::sector_tracker::SectorTracker::default();
    let mut finish_tracker = telemetry::finish_tracker::FinishTracker::default();
    let mut session_transition = telemetry::session_transition::SessionTransitionDetector::default();
    let mut recorder = telemetry::track_recorder::TrackRecorder::new(cache_dir);
    let mut synthetic = SyntheticSubSessionId::default();
    let mut frame: u64 = 0;

    loop {
        match client.wait_for_frame() {
            Ok(()) => {}
            Err(e) => {
                let is_idle = e.to_string().contains("sim idle");
                if is_idle {
                    synthetic.reset();
                    session_transition = telemetry::session_transition::SessionTransitionDetector::default();
                    continue;
                }
                synthetic.reset();
                return Err(e.into());
            }
        }

        let tel = telemetry::TelemetrySnapshot::build(&client).context("telemetry build")?;
        let player_car_idx = tel.player_car_idx;
        let session_num = tel.session_num;
        let _ = tel_tx.send(Some(tel));

        // Sector tracker runs every frame for 60 Hz accuracy.
        if let Some(ref y) = yaml_cache {
            if let Err(e) = sector_tracker.update(&client, y) {
                log::warn!("sector tracker: {e}");
            }
        }

        // Every 15 frames ≈ 250 ms at 60 Hz → 4 Hz standings + session-info
        if frame % 15 == 0 {
            let cur = client.session_info_update();
            if cur != last_si_update || yaml_cache.is_none() {
                let yaml = decode_and_parse(client.session_info_bytes()).context("session_info YAML decode")?;
                let _ = si_tx.send(Some(yaml.clone()));
                yaml_cache = Some(yaml);
                last_si_update = cur;
            }
            if let Some(ref y) = yaml_cache {
                let sub_id = synthetic.resolve(
                    y.weekend_info.sub_session_id,
                    &y.weekend_info,
                    &y.session_info.sessions,
                );
                pit_tracker.update(&client, sub_id).context("pit_tracker update")?;
                finish_tracker.observe(&client, sub_id, session_num)
                    .context("finish_tracker observe")?;
                let snap = telemetry::StandingsSnapshot::build(
                    &client, y, &pit_tracker, &sector_tracker, &mut finish_tracker,
                )
                .context("standings build")?;
                // Drain completed laps (sector_tracker still tracks these for SectorTracker state).
                let _ = sector_tracker.drain_completed_laps();
                // Detect session transitions for synthetic ID bookkeeping.
                session_transition.observe(session_num);
                finish_tracker.checkered_edge_fired();
                let _ = std_tx.send(Some(snap));
            }
        }

        // TrackRecorder runs every frame (needs 60Hz position data for integration).
        if let Some(ref y) = yaml_cache {
            if let Err(e) = recorder.update(&client, y) {
                log::warn!("track recorder: {e}");
            }
            // Push TrackMap at ~15 Hz (every 4 frames).
            if frame % 4 == 0 {
                match telemetry::track_map::TrackMapSnapshot::build(&client, y, &recorder, player_car_idx) {
                    Ok(snap) => { let _ = tm_tx.send(Some(snap)); }
                    Err(e) => log::warn!("track_map build: {e}"),
                }
            }
        }

        frame = frame.wrapping_add(1);
    }
}

/// Determines the local LAN IP via the routing table (no packets sent).
/// Uses RFC 5737 TEST-NET (192.0.2.0/24) — never routed on the real internet,
/// avoids "hardcoded external IP" heuristics in AV scanners.
fn detect_lan_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:1").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

fn log_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("bridge.log")))
        .unwrap_or_else(|| std::path::PathBuf::from("bridge.log"))
}

fn init_logging() {
    use simplelog::{CombinedLogger, ConfigBuilder, LevelFilter, WriteLogger};
    #[cfg(not(all(target_os = "windows", not(debug_assertions))))]
    use simplelog::{ColorChoice, TermLogger, TerminalMode};

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

    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = Vec::new();
    #[cfg(not(all(target_os = "windows", not(debug_assertions))))]
    loggers.push(TermLogger::new(level, cfg.clone(), TerminalMode::Mixed, ColorChoice::Auto));
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
