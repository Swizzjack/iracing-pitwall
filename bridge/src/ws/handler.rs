//! HTTP + WebSocket server (axum router, rust-embed for static dashboard).
//!
//! Routes:
//! - `GET /ws`        → WebSocket upgrade, latest-wins fan-out from three watch channels
//! - `GET /*path`     → embedded `dashboard/dist/` (SPA fallback to `index.html`)

use std::net::SocketAddr;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use rust_embed::RustEmbed;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, watch};

use crate::error::{BridgeError, Result};
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::telemetry::{StandingsSnapshot, TelemetrySnapshot, TrackMapSnapshot};
use crate::update::UpdateInfo;
use crate::ws::client::ClientMessage;
use crate::ws::lifecycle::ClientTracker;
use crate::ws::protocol::ServerMessage;

#[derive(RustEmbed)]
#[folder = "../dashboard/dist"]
struct DashboardAssets;

#[derive(Clone)]
pub struct BridgeState {
    pub telemetry: watch::Receiver<Option<TelemetrySnapshot>>,
    pub standings: watch::Receiver<Option<StandingsSnapshot>>,
    pub session_info: watch::Receiver<Option<SessionInfoYaml>>,
    pub track_map: watch::Receiver<Option<TrackMapSnapshot>>,
    pub update: watch::Receiver<Option<UpdateInfo>>,
    pub command: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub clients: ClientTracker,
    pub lan_url: Option<String>,
}

pub async fn bind(port: u16) -> Result<(SocketAddr, TcpListener)> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    log::info!("HTTP+WS server listening on http://{local_addr}");
    Ok((local_addr, listener))
}

pub async fn serve(
    listener: TcpListener,
    state: BridgeState,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback(static_handler)
        .with_state(state);

    let server = async move { axum::serve(listener, app).await };
    tokio::select! {
        r = server => r?,
        _ = shutdown_rx => {
            log::info!("ws server: shutdown signal received, dropping listener");
        }
    }
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<BridgeState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: BridgeState) {
    let _guard = state.clients.guard();
    log::info!("ws client connected");
    if let Err(e) = handle_socket_inner(socket, state).await {
        log::warn!("ws client disconnected: {e}");
    } else {
        log::info!("ws client closed");
    }
}

type WsSink = SplitSink<WebSocket, Message>;

async fn handle_socket_inner(socket: WebSocket, state: BridgeState) -> Result<()> {
    let (mut sink, mut stream) = socket.split();
    let cmd_tx = state.command.clone();

    send_msg(
        &mut sink,
        &ServerMessage::Hello {
            bridge_version: env!("CARGO_PKG_VERSION").to_string(),
            lan_url: state.lan_url.clone(),
        },
    )
    .await?;

    let mut tel_rx = state.telemetry;
    let mut std_rx = state.standings;
    let mut si_rx = state.session_info;
    let mut tm_rx = state.track_map;
    let mut upd_rx = state.update;

    // Initial replay — clone immediately so the watch::Ref guard drops before any await.
    let init_tel = tel_rx.borrow_and_update().clone();
    let init_std = std_rx.borrow_and_update().clone();
    let init_si = si_rx.borrow_and_update().clone();
    let init_tm = tm_rx.borrow_and_update().clone();
    let init_upd = upd_rx.borrow_and_update().clone();

    if let Some(s) = init_tel {
        send_msg(&mut sink, &ServerMessage::Telemetry { snapshot: s }).await?;
    }
    if let Some(s) = init_std {
        send_msg(&mut sink, &ServerMessage::Standings { snapshot: s }).await?;
    }
    if let Some(i) = init_si {
        send_msg(&mut sink, &ServerMessage::SessionInfo { info: i }).await?;
    }
    if let Some(s) = init_tm {
        send_msg(&mut sink, &ServerMessage::TrackMap { snapshot: s }).await?;
    }
    if let Some(u) = init_upd {
        send_msg(&mut sink, &ServerMessage::UpdateAvailable {
            latest_version: u.latest_version,
            release_url: u.release_url,
        }).await?;
    }

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    None | Some(Err(_)) => return Ok(()),
                    Some(Ok(Message::Text(txt))) => {
                        match serde_json::from_str::<ClientMessage>(&txt) {
                            Ok(cmd) => { let _ = cmd_tx.send(cmd); }
                            Err(e) => log::warn!("ws: ignoring bad client message: {e}"),
                        }
                    }
                    Some(Ok(_)) => {} // ping/pong/binary/close
                }
            }
            r = tel_rx.changed() => {
                r.map_err(|_| BridgeError::WebSocket("telemetry channel closed".into()))?;
                let snap = tel_rx.borrow_and_update().clone();
                if let Some(s) = snap {
                    send_msg(&mut sink, &ServerMessage::Telemetry { snapshot: s }).await?;
                }
            }
            r = std_rx.changed() => {
                r.map_err(|_| BridgeError::WebSocket("standings channel closed".into()))?;
                let snap = std_rx.borrow_and_update().clone();
                if let Some(s) = snap {
                    send_msg(&mut sink, &ServerMessage::Standings { snapshot: s }).await?;
                }
            }
            r = si_rx.changed() => {
                r.map_err(|_| BridgeError::WebSocket("session_info channel closed".into()))?;
                let info = si_rx.borrow_and_update().clone();
                if let Some(i) = info {
                    send_msg(&mut sink, &ServerMessage::SessionInfo { info: i }).await?;
                }
            }
            r = tm_rx.changed() => {
                r.map_err(|_| BridgeError::WebSocket("track_map channel closed".into()))?;
                let snap = tm_rx.borrow_and_update().clone();
                if let Some(s) = snap {
                    send_msg(&mut sink, &ServerMessage::TrackMap { snapshot: s }).await?;
                }
            }
            r = upd_rx.changed() => {
                r.map_err(|_| BridgeError::WebSocket("update channel closed".into()))?;
                let upd = upd_rx.borrow_and_update().clone();
                if let Some(u) = upd {
                    send_msg(&mut sink, &ServerMessage::UpdateAvailable {
                        latest_version: u.latest_version,
                        release_url: u.release_url,
                    }).await?;
                }
            }
        }
    }
}

async fn send_msg(sink: &mut WsSink, msg: &ServerMessage) -> Result<()> {
    let text = serde_json::to_string(msg)?;
    sink.send(Message::Text(text)).await?;
    Ok(())
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = DashboardAssets::get(path) {
        let mime = file.metadata.mimetype().to_owned();
        return ([(header::CONTENT_TYPE, mime)], file.data.into_owned()).into_response();
    }

    // SPA fallback: any unknown path serves index.html
    if let Some(file) = DashboardAssets::get("index.html") {
        return (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            file.data.into_owned(),
        )
            .into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}
