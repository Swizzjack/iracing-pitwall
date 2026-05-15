//! iRacing Pitwall — Bridge
//!
//! Tokio runtime, HTTP+WebSocket server, iRacing SDK reader.

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#![allow(dead_code)]

mod config;
mod error;
mod iracing_api;
mod iracing_sdk;
mod paths;
mod persistence;
mod results;
mod telemetry;
mod ws;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
#[cfg(windows)]
use anyhow::Context;
use tokio::sync::{broadcast, mpsc, oneshot, watch};

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

    let lan_url = detect_lan_ip().map(|ip| format!("http://{}:{}/", ip, cfg.ws_port));
    if let Some(ref url) = lan_url {
        log::info!("LAN access: {url}");
    }

    // ── Results subsystem ───────────────────────────────────────────────
    let data_dir = paths::data_dir();
    let db = persistence::Db::open(&data_dir.join("results.sqlite"))
        .map_err(|e| { log::warn!("db open failed: {e}"); e })?;
    let api = iracing_api::ApiClient::new(data_dir.join("auth.json"))
        .map_err(|e| { log::warn!("api client init failed: {e}"); e })?;
    let (finish_tx, finish_rx) = mpsc::channel::<results::SubSessionEnd>(32);
    let (results_push_tx, _) = broadcast::channel::<ws::ServerMessage>(64);
    // Live capture: SDK loop builds snapshot data synchronously; async task writes to DB.
    let (live_capture_tx, live_capture_rx) =
        mpsc::channel::<results::live_capture::LiveCapture>(8);
    // Lap buffer: receives per-car lap completions and batch-writes to live_session_laps.
    let (lap_buffer_tx, lap_buffer_rx) =
        mpsc::channel::<results::lap_buffer::LapBufferMsg>(256);

    {
        let db2 = db.clone();
        let api2 = api.clone();
        let push2 = results_push_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = results::run(db2, api2, finish_rx, push2).await {
                log::error!("results service: {e}");
            }
        });
    }
    {
        let db_cap = db.clone();
        let push_cap = results_push_tx.clone();
        tokio::spawn(async move {
            results::live_capture::run(db_cap, push_cap, live_capture_rx).await;
        });
    }
    {
        let db_lap = db.clone();
        tokio::spawn(async move {
            results::lap_buffer::run(db_lap, lap_buffer_rx).await;
        });
    }

    let (clients, count_rx) = ws::ClientTracker::new();
    let state = ws::BridgeState {
        telemetry: tel_rx,
        standings: std_rx,
        session_info: si_rx,
        track_map: tm_rx,
        clients,
        lan_url,
        db,
        api,
        finish_tx,
        results_push: results_push_tx,
    };

    // Shared state for Ctrl-C final capture: SDK loop writes latest (yaml, snap) here.
    let last_state: Arc<Mutex<Option<(iracing_sdk::types::SessionInfoYaml, telemetry::StandingsSnapshot)>>> =
        Arc::new(Mutex::new(None));
    let live_capture_tx_ctrlc = live_capture_tx.clone();

    #[cfg(windows)]
    {
        let finish_tx_sdk = state.finish_tx.clone();
        let last_state_sdk = last_state.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = sdk_loop(tel_tx, std_tx, si_tx, tm_tx, finish_tx_sdk, live_capture_tx, lap_buffer_tx, last_state_sdk) {
                log::error!("sdk_loop terminated: {e}");
            }
        });
    }

    #[cfg(not(windows))]
    {
        log::warn!("Non-Windows build: iRacing SDK reader disabled.");
        drop((tel_tx, std_tx, si_tx, tm_tx, live_capture_tx, lap_buffer_tx, last_state.clone()));
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
                log::warn!("failed to open browser: {e}");
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
            log::info!("ctrl-c received, capturing final state");
            do_final_capture(&last_state, &live_capture_tx_ctrlc);
        }
        r = ws_task => {
            log::info!("ws task ended, capturing final state");
            do_final_capture(&last_state, &live_capture_tx_ctrlc);
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

/// Shared final-capture logic used by both Ctrl-C and WS-lifecycle-shutdown paths.
fn do_final_capture(
    last_state: &Arc<Mutex<Option<(iracing_sdk::types::SessionInfoYaml, telemetry::StandingsSnapshot)>>>,
    tx: &mpsc::Sender<results::live_capture::LiveCapture>,
) {
    if let Ok(guard) = last_state.lock() {
        if let Some((ref yaml, ref snap)) = *guard {
            match results::live_capture::capture_subsession(yaml, snap, true) {
                Ok(capture) => {
                    let _ = tx.try_send(capture);
                    std::thread::sleep(Duration::from_millis(600));
                }
                Err(e) => log::warn!("final capture failed: {e}"),
            }
        }
    }
}

/// Blocking SDK loop — runs in spawn_blocking, owns the IRacingClient for its lifetime.
/// Retries indefinitely until iRacing is running, reconnects after disconnect.
#[cfg(windows)]
fn sdk_loop(
    tel_tx: watch::Sender<Option<telemetry::TelemetrySnapshot>>,
    std_tx: watch::Sender<Option<telemetry::StandingsSnapshot>>,
    si_tx: watch::Sender<Option<iracing_sdk::types::SessionInfoYaml>>,
    tm_tx: watch::Sender<Option<telemetry::TrackMapSnapshot>>,
    finish_tx: mpsc::Sender<results::SubSessionEnd>,
    live_capture_tx: mpsc::Sender<results::live_capture::LiveCapture>,
    lap_buffer_tx: mpsc::Sender<results::lap_buffer::LapBufferMsg>,
    last_state: Arc<Mutex<Option<(iracing_sdk::types::SessionInfoYaml, telemetry::StandingsSnapshot)>>>,
) -> Result<()> {
    const RETRY_DELAY: Duration = Duration::from_secs(3);

    loop {
        match connect_and_run(&tel_tx, &std_tx, &si_tx, &tm_tx, &finish_tx, &live_capture_tx, &lap_buffer_tx, &last_state) {
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
    finish_tx: &mpsc::Sender<results::SubSessionEnd>,
    live_capture_tx: &mpsc::Sender<results::live_capture::LiveCapture>,
    lap_buffer_tx: &mpsc::Sender<results::lap_buffer::LapBufferMsg>,
    last_state: &Arc<Mutex<Option<(iracing_sdk::types::SessionInfoYaml, telemetry::StandingsSnapshot)>>>,
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
    let mut last_snap: Option<telemetry::StandingsSnapshot> = None;
    let mut recorder = telemetry::track_recorder::TrackRecorder::new(cache_dir);
    let mut synthetic = SyntheticSubSessionId::default();
    let mut frame: u64 = 0;
    // For SubSessionID-change detection (user leaves server, enters a different session).
    let mut last_sub_id: Option<i64> = None;
    // For periodic live-persist: save current state every LIVE_PERSIST_INTERVAL seconds.
    const LIVE_PERSIST_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
    let mut last_live_persist_at = std::time::Instant::now();

    loop {
        match client.wait_for_frame() {
            Ok(()) => {}
            Err(e) => {
                let is_idle = e.to_string().contains("sim idle");
                if is_idle {
                    // Sim is running but player returned to main menu or loading screen.
                    // Do a final capture if we have state, then keep looping.
                    if let (Some(ref snap), Some(ref yaml)) = (&last_snap, &yaml_cache) {
                        let sub_id = synthetic.resolve(
                            yaml.weekend_info.sub_session_id,
                            &yaml.weekend_info,
                            &yaml.session_info.sessions,
                        );
                        if sub_id != 0 {
                            log::info!(
                                "sdk_loop: sim idle — capturing subsession={sub_id} session_num={}",
                                snap.session_num
                            );
                            let _ = lap_buffer_tx.blocking_send(
                                results::lap_buffer::LapBufferMsg::Flush { sub_session_id: sub_id },
                            );
                            match results::live_capture::capture_subsession(yaml, snap, true) {
                                Ok(capture) => { let _ = live_capture_tx.blocking_send(capture); }
                                Err(ce) => log::warn!("idle capture failed: {ce}"),
                            }
                        }
                        // Reset sub_id tracking so a subsequent reconnect is treated as fresh.
                        last_sub_id = None;
                        synthetic.reset();
                        session_transition = telemetry::session_transition::SessionTransitionDetector::default();
                    }
                    continue;
                }
                // Real disconnect (iRacing closed / connected-bit cleared).
                if let (Some(ref snap), Some(ref yaml)) = (&last_snap, &yaml_cache) {
                    let sub_id = synthetic.resolve(
                        yaml.weekend_info.sub_session_id,
                        &yaml.weekend_info,
                        &yaml.session_info.sessions,
                    );
                    if sub_id != 0 {
                        log::info!(
                            "sdk_loop: disconnect — capturing subsession={sub_id} session_num={}",
                            snap.session_num
                        );
                        let _ = lap_buffer_tx.blocking_send(
                            results::lap_buffer::LapBufferMsg::Flush { sub_session_id: sub_id },
                        );
                        match results::live_capture::capture_subsession(yaml, snap, true) {
                            Ok(capture) => { let _ = live_capture_tx.blocking_send(capture); }
                            Err(ce) => log::warn!("disconnect capture failed: {ce}"),
                        }
                    }
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
                // Build snapshot before sending so we can use it for live capture.
                let snap = telemetry::StandingsSnapshot::build(
                    &client, y, &pit_tracker, &sector_tracker, &mut finish_tracker,
                )
                .context("standings build")?;
                // Forward any completed laps to the buffer (all sessions, not just race).
                let completions = sector_tracker.drain_completed_laps();
                if !completions.is_empty() {
                    let _ = lap_buffer_tx.blocking_send(results::lap_buffer::LapBufferMsg::Batch {
                        sub_session_id: sub_id,
                        simsession_num: session_num,
                        completions,
                    });
                }
                // Detect SubSessionID change (user left server, entered a new session).
                if let Some(prev_sub) = last_sub_id {
                    if prev_sub != sub_id {
                        log::info!("sdk_loop: sub_session_id changed {prev_sub}→{sub_id} — capturing previous");
                        let _ = lap_buffer_tx.blocking_send(
                            results::lap_buffer::LapBufferMsg::Flush { sub_session_id: prev_sub },
                        );
                        if let Some(ref prev_snap) = last_snap {
                            match results::live_capture::capture_subsession(y, prev_snap, true) {
                                Ok(capture) => { let _ = live_capture_tx.blocking_send(capture); }
                                Err(e) => log::warn!("subsession-change capture failed: {e}"),
                            }
                        }
                        session_transition = telemetry::session_transition::SessionTransitionDetector::default();
                    }
                }
                last_sub_id = Some(sub_id);
                // Detect Practice/Qualify → next-session transition and capture the finished segment.
                if let Some(prev_num) = session_transition.observe(session_num) {
                    log::info!("sdk_loop: session transition {prev_num}→{session_num} — capturing previous segment");
                    let _ = lap_buffer_tx.blocking_send(results::lap_buffer::LapBufferMsg::Flush {
                        sub_session_id: sub_id,
                    });
                    // Use last tick's snapshot (built under prev_num) + current YAML (which
                    // now has ResultsPositions for the finished session where available).
                    if let Some(ref prev_snap) = last_snap {
                        match results::live_capture::capture_subsession(y, prev_snap, true) {
                            Ok(capture) => { let _ = live_capture_tx.blocking_send(capture); }
                            Err(e) => log::warn!("live_capture (transition): {e}"),
                        }
                    }
                }
                // Trigger results on checkered-flag rising edge.
                if let Some(fired_sub_id) = finish_tracker.checkered_edge_fired() {
                    log::info!("sdk_loop: checkered edge — queuing results fetch for subsession {fired_sub_id}");
                    let _ = lap_buffer_tx.blocking_send(results::lap_buffer::LapBufferMsg::Flush {
                        sub_session_id: fired_sub_id,
                    });
                    match results::live_capture::capture_subsession(y, &snap, true) {
                        Ok(capture) => { let _ = live_capture_tx.blocking_send(capture); }
                        Err(e) => log::warn!("live_capture build failed: {e}"),
                    }
                    // API fetch path (dormant until OAuth re-opens).
                    let _ = finish_tx.blocking_send(results::SubSessionEnd { sub_session_id: fired_sub_id });
                }
                // Periodic live-persist: write current state every LIVE_PERSIST_INTERVAL seconds.
                let now_instant = std::time::Instant::now();
                if sub_id != 0 && now_instant.duration_since(last_live_persist_at) >= LIVE_PERSIST_INTERVAL {
                    last_live_persist_at = now_instant;
                    match results::live_capture::capture_subsession(y, &snap, false) {
                        Ok(capture) => { let _ = live_capture_tx.try_send(capture); }
                        Err(e) => log::warn!("live-persist capture failed: {e}"),
                    }
                }
                // Always keep last_state current for Ctrl-C / WS-shutdown final capture.
                if sub_id != 0 {
                    if let Ok(mut guard) = last_state.lock() {
                        *guard = Some((y.clone(), snap.clone()));
                    }
                }
                last_snap = Some(snap.clone());
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

/// Determines the local LAN IP by routing towards a public address (no packets sent).
fn detect_lan_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

fn log_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("bridge.log")))
        .unwrap_or_else(|| PathBuf::from("bridge.log"))
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
