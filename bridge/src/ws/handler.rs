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
use tokio::sync::{broadcast, mpsc, oneshot, watch};

use crate::error::{BridgeError, Result};
use crate::iracing_api::ApiClient;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::persistence::Db;
use crate::results::SubSessionEnd;
use crate::telemetry::{StandingsSnapshot, TelemetrySnapshot, TrackMapSnapshot};
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
    pub clients: ClientTracker,
    pub lan_url: Option<String>,
    pub db: Db,
    pub api: ApiClient,
    /// Sender for session-end events → results service.
    pub finish_tx: mpsc::Sender<SubSessionEnd>,
    /// Receiver for push events from the results service (broadcast).
    pub results_push: broadcast::Sender<ServerMessage>,
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

    send_msg(
        &mut sink,
        &ServerMessage::Hello {
            bridge_version: env!("CARGO_PKG_VERSION").to_string(),
            lan_url: state.lan_url.clone(),
        },
    )
    .await?;

    // Send current OAuth link status immediately after Hello.
    {
        let (linked, member_name, cust_id) = state.api.get_linked_info().await;
        send_msg(&mut sink, &ServerMessage::OAuthStatus { linked, member_name, cust_id }).await?;
    }

    let mut tel_rx = state.telemetry;
    let mut std_rx = state.standings;
    let mut si_rx = state.session_info;
    let mut tm_rx = state.track_map;

    // Initial replay — clone immediately so the watch::Ref guard drops before any await.
    let init_tel = tel_rx.borrow_and_update().clone();
    let init_std = std_rx.borrow_and_update().clone();
    let init_si = si_rx.borrow_and_update().clone();
    let init_tm = tm_rx.borrow_and_update().clone();

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

    let db = state.db.clone();
    let api = state.api.clone();
    let finish_tx = state.finish_tx.clone();
    let mut results_rx = state.results_push.subscribe();

    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    None | Some(Err(_)) => return Ok(()),
                    Some(Ok(Message::Text(text))) => {
                        handle_client_msg(&text, &mut sink, &db, &api, &finish_tx).await;
                    }
                    Some(Ok(_)) => {}
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
            push = results_rx.recv() => {
                match push {
                    Ok(msg) => send_msg(&mut sink, &msg).await?,
                    Err(broadcast::error::RecvError::Lagged(_)) => {} // skip missed pushes
                    Err(broadcast::error::RecvError::Closed) => {} // results service stopped
                }
            }
        }
    }
}

async fn handle_client_msg(
    text: &str,
    sink: &mut WsSink,
    db: &Db,
    api: &ApiClient,
    finish_tx: &mpsc::Sender<SubSessionEnd>,
) {
    let msg = match serde_json::from_str::<ClientMessage>(text) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("ws: invalid client message: {e} — {text}");
            return;
        }
    };

    match msg {
        ClientMessage::StartOAuth => {
            match crate::iracing_api::auth::start_flow().await {
                Ok((url, token_handle)) => {
                    let _ = send_msg(sink, &ServerMessage::OAuthUrl { url }).await;
                    let api = api.clone();
                    tokio::spawn(async move {
                        match token_handle.await {
                            Ok(Ok(tokens)) => {
                                if let Err(e) = api.store_tokens(tokens).await {
                                    log::warn!("oauth: store_tokens failed: {e}");
                                } else {
                                    let (linked, name, cust_id) = api.get_linked_info().await;
                                    log::info!("oauth: linked as {:?} (cust_id={:?})", name, cust_id);
                                    // We can't push to this specific client here — that's OK,
                                    // the client will query OAuthStatus on next load.
                                    let _ = (linked, name, cust_id);
                                }
                            }
                            Ok(Err(e)) => log::warn!("oauth: token exchange failed: {e}"),
                            Err(e) => log::warn!("oauth: task panicked: {e}"),
                        }
                    });
                }
                Err(e) => log::warn!("oauth: start_flow failed: {e}"),
            }
        }

        ClientMessage::QueryResults { filter } => {
            let result = db
                .with(move |c| crate::persistence::queries::query_sessions(c, &filter))
                .await;
            match result {
                Ok((sessions, total)) => {
                    let _ = send_msg(sink, &ServerMessage::ResultsList { sessions, total }).await;
                }
                Err(e) => log::warn!("ws: query_sessions failed: {e}"),
            }
        }

        ClientMessage::QueryResultDetail { sub_session_id } => {
            let result = db
                .with(move |c| crate::persistence::queries::get_session_detail(c, sub_session_id))
                .await;
            match result {
                Ok(Some(session)) => {
                    let _ = send_msg(sink, &ServerMessage::ResultDetail { session }).await;
                }
                Ok(None) => log::info!("ws: no detail for subsession {sub_session_id}"),
                Err(e) => log::warn!("ws: get_session_detail failed: {e}"),
            }
        }

        ClientMessage::QueryFilterOptions => {
            let result = db
                .with(|c| crate::persistence::queries::get_filter_options(c))
                .await;
            match result {
                Ok(options) => {
                    let _ = send_msg(sink, &ServerMessage::FilterOptions { options }).await;
                }
                Err(e) => log::warn!("ws: get_filter_options failed: {e}"),
            }
        }

        ClientMessage::TriggerFetch { sub_session_id } => {
            log::info!("ws: manual fetch trigger for subsession {sub_session_id}");
            let _ = finish_tx
                .send(SubSessionEnd { sub_session_id })
                .await;
        }

        ClientMessage::QueryLaps { sub_session_id, car_idx } => {
            let result = db
                .with(move |c| crate::persistence::queries::get_laps(c, sub_session_id, car_idx))
                .await;
            match result {
                Ok(laps) => {
                    let _ = send_msg(sink, &ServerMessage::LapsList { laps }).await;
                }
                Err(e) => log::warn!("ws: get_laps failed: {e}"),
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
