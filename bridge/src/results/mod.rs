//! Post-session results fetcher and local store orchestrator.

pub mod converter;
pub mod finalizer;
pub mod lap_buffer;
pub mod live_capture;
pub mod retry_queue;

use anyhow::Result;
use tokio::sync::{broadcast, mpsc};

use crate::iracing_api::ApiClient;
use crate::persistence::Db;
use crate::ws::protocol::ServerMessage;

pub struct SubSessionEnd {
    pub sub_session_id: i64,
}

/// Entry point — spawn as a long-running tokio task.
pub async fn run(
    db: Db,
    api: ApiClient,
    finish_rx: mpsc::Receiver<SubSessionEnd>,
    push_tx: broadcast::Sender<ServerMessage>,
) -> Result<()> {
    let (fetch_tx, fetch_rx) = mpsc::channel::<i64>(64);

    // Retry queue loop runs concurrently.
    let retry_db = db.clone();
    let retry_api = api.clone();
    let retry_push = push_tx.clone();
    let retry_fetch_tx = fetch_tx.clone();
    tokio::spawn(retry_queue::run(retry_db, retry_api, retry_push, retry_fetch_tx));

    // Finalizer: receives session-end events from the SDK loop (via blocking_send)
    // and dispatches them to the fetch worker after a debounce.
    tokio::spawn(finalizer::run(finish_rx, fetch_tx));

    // Fetch worker: pulls SubSessionIDs and fetches from the API.
    fetch_worker(db, api, push_tx, fetch_rx).await;
    Ok(())
}

async fn fetch_worker(
    db: Db,
    api: ApiClient,
    push_tx: broadcast::Sender<ServerMessage>,
    mut rx: mpsc::Receiver<i64>,
) {
    while let Some(sub_id) = rx.recv().await {
        // Skip if already in DB.
        let exists = db.with(move |c| crate::persistence::queries::session_exists(c, sub_id)).await;
        match exists {
            Ok(true) => {
                log::info!("results: subsession {sub_id} already in db, skipping");
                continue;
            }
            Err(e) => log::warn!("results: db check failed: {e}"),
            Ok(false) => {}
        }

        log::info!("results: fetching subsession {sub_id}");
        match fetch_and_store(sub_id, &db, &api).await {
            Ok(()) => {
                log::info!("results: stored subsession {sub_id}");
                let _ = push_tx.send(ServerMessage::ResultsUpdated { sub_session_id: sub_id });
            }
            Err(e) => {
                log::warn!("results: fetch failed for {sub_id}: {e}");
                // Enqueue for retry.
                let now = unix_now() + 30_000;
                if let Err(e) = db.with(move |c| {
                    crate::persistence::queries::enqueue_pending(c, sub_id, now)
                }).await {
                    log::warn!("results: enqueue_pending failed: {e}");
                }
            }
        }
    }
}

pub async fn fetch_and_store(sub_id: i64, db: &Db, api: &ApiClient) -> Result<()> {
    let result = api.get_subsession_result(sub_id).await?;
    let raw_json = serde_json::to_string(&result)?;
    let (session_row, segment_rows, result_rows) = converter::convert(sub_id, result, raw_json)?;

    db.with(move |c| {
        crate::persistence::queries::upsert_subsession(c, &session_row)?;
        for seg in &segment_rows {
            crate::persistence::queries::upsert_segment(c, seg)?;
        }
        crate::persistence::queries::insert_result_rows(c, &result_rows)
    })
    .await?;
    Ok(())
}

/// Unix timestamp in **milliseconds**. All DB time fields use ms to match
/// JavaScript's `new Date(ms)` convention.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
