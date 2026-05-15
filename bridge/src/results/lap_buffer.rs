//! Async buffer that collects LapCompletion events and batch-writes them to SQLite.
//!
//! Flushes when (a) ≥50 laps accumulated, (b) 30 s elapsed, or (c) explicit Flush signal.

use std::time::Duration;

use tokio::sync::mpsc;

use crate::persistence::{
    queries::{insert_laps, LapRow},
    Db,
};
use crate::telemetry::sector_tracker::LapCompletion;

const FLUSH_INTERVAL: Duration = Duration::from_secs(30);
const FLUSH_LAP_COUNT: usize = 50;

pub enum LapBufferMsg {
    /// Batch of new lap completions from the SDK loop.
    Batch {
        sub_session_id: i64,
        simsession_num: i32,
        completions: Vec<LapCompletion>,
    },
    /// Force-flush all buffered laps for the given sub-session (called at checkered edge).
    Flush { sub_session_id: i64 },
}

/// Async task: buffers lap completions and writes them to `live_session_laps` in batches.
pub async fn run(db: Db, mut rx: mpsc::Receiver<LapBufferMsg>) {
    let mut buffer: Vec<LapRow> = Vec::new();

    loop {
        match tokio::time::timeout(FLUSH_INTERVAL, rx.recv()).await {
            Err(_) => {
                // Periodic timeout: flush whatever we have.
                if !buffer.is_empty() {
                    flush_all(&db, &mut buffer).await;
                }
            }
            Ok(None) => break, // channel closed — final flush below
            Ok(Some(LapBufferMsg::Batch { sub_session_id, simsession_num, completions })) => {
                for c in completions {
                    buffer.push(lap_completion_to_row(sub_session_id, simsession_num, c));
                }
                if buffer.len() >= FLUSH_LAP_COUNT {
                    flush_all(&db, &mut buffer).await;
                }
            }
            Ok(Some(LapBufferMsg::Flush { sub_session_id })) => {
                let to_flush: Vec<LapRow> = buffer
                    .drain(..)
                    .filter(|r| r.sub_session_id == sub_session_id)
                    .collect();
                if !to_flush.is_empty() {
                    write_rows(&db, to_flush).await;
                }
            }
        }
    }

    if !buffer.is_empty() {
        flush_all(&db, &mut buffer).await;
    }
}

async fn flush_all(db: &Db, buffer: &mut Vec<LapRow>) {
    let rows = std::mem::take(buffer);
    write_rows(db, rows).await;
}

async fn write_rows(db: &Db, rows: Vec<LapRow>) {
    if let Err(e) = db.with(move |c| insert_laps(c, &rows)).await {
        log::warn!("lap_buffer: write failed: {e}");
    }
}

fn lap_completion_to_row(
    sub_session_id: i64,
    simsession_num: i32,
    c: LapCompletion,
) -> LapRow {
    let sectors_json = if c.sectors.is_empty() {
        None
    } else {
        serde_json::to_string(&c.sectors).ok()
    };

    LapRow {
        sub_session_id,
        simsession_num,
        car_idx: c.car_idx,
        lap_num: c.lap_num,
        cust_id: None,
        lap_time_sec: c.lap_time_sec,
        sectors_json,
        valid: c.valid,
        in_lap: c.in_lap,
        out_lap: false,
        incidents_delta: None,
        position_at_end: None,
        air_temp: c.air_temp,
        track_temp: c.track_temp,
        track_wetness: None,
        rubber_state: None,
        session_time: Some(c.session_time),
    }
}
