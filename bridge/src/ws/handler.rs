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
use tokio::sync::watch;

use crate::error::{BridgeError, Result};
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::telemetry::{StandingsSnapshot, TelemetrySnapshot};
use crate::ws::protocol::ServerMessage;

#[derive(RustEmbed)]
#[folder = "../dashboard/dist"]
struct DashboardAssets;

#[derive(Clone)]
pub struct BridgeState {
    pub telemetry: watch::Receiver<Option<TelemetrySnapshot>>,
    pub standings: watch::Receiver<Option<StandingsSnapshot>>,
    pub session_info: watch::Receiver<Option<SessionInfoYaml>>,
}

pub async fn serve(port: u16, state: BridgeState) -> Result<()> {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback(static_handler)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    log::info!("HTTP+WS server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<BridgeState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: BridgeState) {
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

    // Drain inbound — clients don't send data, but pings/close frames must be consumed.
    tokio::spawn(async move { while stream.next().await.is_some() {} });

    send_msg(
        &mut sink,
        &ServerMessage::Hello {
            bridge_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    )
    .await?;

    let mut tel_rx = state.telemetry;
    let mut std_rx = state.standings;
    let mut si_rx = state.session_info;

    // Initial replay — clone immediately so the watch::Ref guard drops before any await.
    let init_tel = tel_rx.borrow_and_update().clone();
    let init_std = std_rx.borrow_and_update().clone();
    let init_si = si_rx.borrow_and_update().clone();

    if let Some(s) = init_tel {
        send_msg(&mut sink, &ServerMessage::Telemetry { snapshot: s }).await?;
    }
    if let Some(s) = init_std {
        send_msg(&mut sink, &ServerMessage::Standings { snapshot: s }).await?;
    }
    if let Some(i) = init_si {
        send_msg(&mut sink, &ServerMessage::SessionInfo { info: i }).await?;
    }

    loop {
        tokio::select! {
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
