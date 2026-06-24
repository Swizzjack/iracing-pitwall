//! iRacing Pitwall — Bridge
//!
//! Tokio runtime, HTTP+WebSocket server, iRacing SDK reader.

#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
// On non-Windows hosts the whole SDK consumer chain hangs off the
// cfg(windows) sdk_loop, so dead-code analysis would flag it all.
// Windows builds (the shipped artifact) still get real dead-code checks.
#![cfg_attr(not(windows), allow(dead_code))]

mod config;
mod error;
mod iracing_sdk;
mod race_engineer;
mod telemetry;
mod update;
mod ws;

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
#[cfg(windows)]
use anyhow::Context;
use tokio::sync::{broadcast, mpsc, oneshot, watch};

/// Checks via TCP connect whether a bridge instance is already running on this port.
/// Faster than port-binding and works without Windows-specific APIs.
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

    // Already an instance running? → open the browser and exit.
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

    // --- Data channels ---
    let (tel_tx, tel_rx) = watch::channel(None::<telemetry::TelemetrySnapshot>);
    let (std_tx, std_rx) = watch::channel(None::<telemetry::StandingsSnapshot>);
    let (si_tx, si_rx) = watch::channel(None::<iracing_sdk::types::SessionInfoYaml>);
    let (tm_tx, tm_rx) = watch::channel(None::<telemetry::TrackMapSnapshot>);
    let (dbg_tx, dbg_rx) = watch::channel(None::<telemetry::SdkDebugSnapshot>);
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<ws::client::ClientMessage>();

    // --- Race Engineer channels ---
    let (eng_state_tx, eng_state_rx) = mpsc::unbounded_channel::<race_engineer::state::EngineerState>();
    let (eng_cmd_tx, eng_cmd_rx) = mpsc::unbounded_channel::<ws::client::ClientMessage>();
    // broadcast capacity: 64 slots (audio clips are infrequent, ~1 every few seconds at most)
    let (eng_audio_tx, _eng_audio_keep_rx) = broadcast::channel::<ws::protocol::ServerMessage>(64);

    // Spawn race engineer async task
    tokio::spawn(race_engineer::run_engineer_task(
        eng_state_rx,
        eng_cmd_rx,
        eng_audio_tx.clone(),
    ));

    // Update check — runs in a background thread so it never delays startup.
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
        // Task ends here. The WS handler tolerates the closed channel, and
        // late-connecting clients still see the last value via the initial
        // `borrow_and_update()` replay.
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
        sdk_debug: dbg_rx,
        update: upd_rx,
        command: cmd_tx,
        engineer_cmd: eng_cmd_tx,
        engineer_audio: eng_audio_tx,
        clients,
        lan_url,
    };

    #[cfg(windows)]
    tokio::task::spawn_blocking(move || {
        if let Err(e) = sdk_loop(tel_tx, std_tx, si_tx, tm_tx, dbg_tx, cmd_rx, eng_state_tx) {
            log::error!("sdk_loop terminated: {e}");
        }
    });

    #[cfg(not(windows))]
    {
        log::warn!("Non-Windows build: iRacing SDK reader disabled.");
        drop((tel_tx, std_tx, si_tx, tm_tx, dbg_tx, cmd_rx, eng_state_tx));
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
    std::process::exit(0);
}

/// Blocking SDK loop — runs in spawn_blocking, owns the IRacingClient for its lifetime.
#[cfg(windows)]
fn sdk_loop(
    tel_tx: watch::Sender<Option<telemetry::TelemetrySnapshot>>,
    std_tx: watch::Sender<Option<telemetry::StandingsSnapshot>>,
    si_tx: watch::Sender<Option<iracing_sdk::types::SessionInfoYaml>>,
    tm_tx: watch::Sender<Option<telemetry::TrackMapSnapshot>>,
    dbg_tx: watch::Sender<Option<telemetry::SdkDebugSnapshot>>,
    mut cmd_rx: mpsc::UnboundedReceiver<ws::client::ClientMessage>,
    eng_state_tx: mpsc::UnboundedSender<race_engineer::state::EngineerState>,
) -> Result<()> {
    const RETRY_DELAY: Duration = Duration::from_secs(3);

    loop {
        match connect_and_run(&tel_tx, &std_tx, &si_tx, &tm_tx, &dbg_tx, &mut cmd_rx, &eng_state_tx) {
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
    dbg_tx: &watch::Sender<Option<telemetry::SdkDebugSnapshot>>,
    cmd_rx: &mut mpsc::UnboundedReceiver<ws::client::ClientMessage>,
    eng_state_tx: &mpsc::UnboundedSender<race_engineer::state::EngineerState>,
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
    let mut p2p_tracker = telemetry::p2p_tracker::P2pTracker::default();
    let mut finish_tracker = telemetry::finish_tracker::FinishTracker::default();
    let mut gap_tracker = telemetry::gap_tracker::GapTracker::default();
    let mut session_transition = telemetry::session_transition::SessionTransitionDetector::default();
    let mut recorder = telemetry::track_recorder::TrackRecorder::new(cache_dir);
    let mut synthetic = SyntheticSubSessionId::default();
    let mut eng_aggregator = race_engineer::state::StateAggregator::default();
    let mut last_standings: Option<telemetry::StandingsSnapshot> = None;
    let mut frame: u64 = 0;
    // Hidden admin/debug view: only build+send the full SDK dump while a
    // client has it open (walking every SDK variable every tick is wasteful
    // when nobody is looking).
    let mut debug_enabled = false;

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

        // Drain dashboard commands (non-blocking)
        while let Ok(cmd) = cmd_rx.try_recv() {
            use ws::client::ClientMessage;
            match cmd {
                ClientMessage::DeleteTrackMap { track_key } => recorder.delete_cached(&track_key),
                ClientMessage::SetSdkDebug { enabled } => {
                    debug_enabled = enabled;
                    if !enabled {
                        let _ = dbg_tx.send(None);
                    }
                }
                _ => {} // Engineer commands are routed directly to eng_cmd_tx in handler
            }
        }

        let tel = telemetry::TelemetrySnapshot::build(&client).context("telemetry build")?;
        let player_car_idx = tel.player_car_idx;
        let session_num = tel.session_num;
        let _ = tel_tx.send(Some(tel.clone()));

        // Sector tracker runs every frame for 60 Hz accuracy.
        if let Some(ref y) = yaml_cache {
            if let Err(e) = sector_tracker.update(&client, y) {
                log::warn!("sector tracker: {e}");
            }
        }

        // Every 6 frames ≈ 10 Hz → engineer state.
        // Reuse the last known standings so session_type stays correct on every tick.
        // Until the first standings snapshot arrives (~first 250 ms) this passes None →
        // session_type = Unknown → all rules skipped (harmless transient).
        if frame % 6 == 0 && frame % 15 != 0 {
            let eng_state = eng_aggregator.build_state(&tel, last_standings.as_ref());
            let _ = eng_state_tx.send(eng_state);
        }

        // Hidden admin/debug view ≈ 4 Hz — only while a client has it open.
        if debug_enabled && frame % 15 == 0 {
            let snap = telemetry::SdkDebugSnapshot::build(&client);
            let _ = dbg_tx.send(Some(snap));
        }

        // Every 15 frames ≈ 4 Hz → standings + session-info
        if frame % 15 == 0 {
            let cur = client.session_info_update();
            if cur != last_si_update || yaml_cache.is_none() {
                let yaml = decode_and_parse(client.session_info_bytes()).context("session_info YAML decode")?;
                let inc_limit = yaml.weekend_info.weekend_options
                    .as_ref()
                    .and_then(|o| o.incident_limit());
                eng_aggregator.set_incident_limit(inc_limit);
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
                p2p_tracker.update(&client, sub_id).context("p2p_tracker update")?;
                finish_tracker.observe(&client, sub_id, session_num)
                    .context("finish_tracker observe")?;
                let snap = telemetry::StandingsSnapshot::build(
                    &client, y, &pit_tracker, &sector_tracker, &p2p_tracker, &finish_tracker,
                    &mut gap_tracker,
                )
                .context("standings build")?;
                let _ = sector_tracker.drain_completed_laps();
                session_transition.observe(session_num);
                finish_tracker.checkered_edge_fired();

                // Engineer state with standings (full data including gaps).
                // Cache snap for reuse in the 10 Hz intermediate ticks.
                let eng_state = eng_aggregator.build_state(&tel, Some(&snap));
                let _ = eng_state_tx.send(eng_state);
                last_standings = Some(snap.clone());

                let _ = std_tx.send(Some(snap));
            }
        }

        // TrackRecorder runs every frame
        if let Some(ref y) = yaml_cache {
            if let Err(e) = recorder.update(&client, y) {
                log::warn!("track recorder: {e}");
            }
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
