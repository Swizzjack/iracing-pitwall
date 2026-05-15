//! Periodic retry worker for sessions that failed to fetch on first attempt.
//!
//! Back-off schedule: 30s → 60s → 2m → 5m → 10m (×10 attempts max, then dropped).

use std::time::Duration;

use tokio::sync::{broadcast, mpsc};

use crate::iracing_api::ApiClient;
use crate::persistence::Db;
use crate::ws::protocol::ServerMessage;

use super::unix_now;

const MAX_ATTEMPTS: i32 = 10;

const BACKOFF_MS: &[i64] = &[30_000, 60_000, 120_000, 300_000, 600_000];

fn next_delay(attempts: i32) -> i64 {
    let idx = (attempts as usize).min(BACKOFF_MS.len() - 1);
    BACKOFF_MS[idx]
}

pub async fn run(
    db: Db,
    api: ApiClient,
    push_tx: broadcast::Sender<ServerMessage>,
    _fetch_tx: mpsc::Sender<i64>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let now = unix_now();
        let due = match db.with(move |c| crate::persistence::queries::get_due_pending(c, now)).await {
            Ok(rows) => rows,
            Err(e) => {
                log::warn!("retry_queue: get_due_pending: {e}");
                continue;
            }
        };

        for (sub_id, attempts) in due {
            if attempts >= MAX_ATTEMPTS {
                log::warn!("retry_queue: giving up on subsession {sub_id} after {attempts} attempts");
                let _ = db.with(move |c| crate::persistence::queries::remove_pending(c, sub_id)).await;
                continue;
            }

            log::info!("retry_queue: retrying subsession {sub_id} (attempt {})", attempts + 1);
            match super::fetch_and_store(sub_id, &db, &api).await {
                Ok(()) => {
                    log::info!("retry_queue: stored subsession {sub_id}");
                    let _ = db.with(move |c| crate::persistence::queries::remove_pending(c, sub_id)).await;
                    let _ = push_tx.send(ServerMessage::ResultsUpdated { sub_session_id: sub_id });
                }
                Err(e) => {
                    let next_attempts = attempts + 1;
                    let next_retry = unix_now() + next_delay(next_attempts);
                    let err_str = e.to_string();
                    log::warn!("retry_queue: attempt {next_attempts} failed for {sub_id}: {e}");
                    let _ = db.with(move |c| {
                        crate::persistence::queries::update_pending_retry(
                            c, sub_id, next_retry, next_attempts, &err_str,
                        )
                    }).await;
                }
            }
        }
    }
}
