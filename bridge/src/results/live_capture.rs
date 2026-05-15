//! Builds a results snapshot from live SDK data at race-end and persists it to SQLite.

use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::{broadcast, mpsc};

use crate::iracing_sdk::types::{DriverEntry, ResultPosition, SessionInfoYaml};
use crate::persistence::{
    queries::{insert_result_rows, upsert_segment, upsert_subsession, LapRow, ResultRow, SegmentRow, SessionRow},
    Db,
};
use crate::telemetry::standings::{StandingEntry, StandingsSnapshot};
use crate::ws::protocol::ServerMessage;

use super::unix_now;

/// Data built synchronously in the SDK loop and sent to the async write task.
pub struct LiveCapture {
    pub session_row: SessionRow,
    pub segment_row: SegmentRow,
    pub result_rows: Vec<ResultRow>,
    pub sub_session_id: i64,
}

/// Build a `LiveCapture` from the current standings snapshot and YAML.
///
/// `is_finalized` = true for Checkered, SessionNum-Transition, SubSession-Change, Disconnect.
/// `is_finalized` = false for periodic live-persist ticks.
pub fn capture_subsession(
    yaml: &SessionInfoYaml,
    snapshot: &StandingsSnapshot,
    is_finalized: bool,
) -> Result<LiveCapture> {
    let weekend = &yaml.weekend_info;
    let sub_session_id = weekend.sub_session_id;

    // car_idx → DriverEntry lookup
    let driver_map: HashMap<i32, &DriverEntry> = yaml
        .driver_info
        .drivers
        .iter()
        .map(|d| (d.car_idx, d))
        .collect();

    // car_idx → ResultPosition lookup (from YAML — populated after checkered)
    let results_by_car: HashMap<i32, &ResultPosition> = yaml
        .session_info
        .sessions
        .iter()
        .find(|s| s.session_num == snapshot.session_num)
        .and_then(|s| s.results_positions.as_ref())
        .map(|rp| rp.iter().map(|r| (r.car_idx, r)).collect())
        .unwrap_or_default();

    let iratings: Vec<i32> = snapshot
        .entries
        .iter()
        .filter(|e| {
            !driver_map
                .get(&e.car_idx)
                .map(|d| d.car_is_pace_car != 0)
                .unwrap_or(false)
        })
        .map(|e| e.irating)
        .collect();
    let sof = if iratings.is_empty() { None } else { Some(compute_sof(&iratings)) };

    let now = unix_now();
    let session_type = snapshot.session_type.clone();
    let simsession_type = session_type_to_label(&session_type).to_string();

    let session_row = SessionRow {
        sub_session_id,
        session_id: Some(weekend.session_id),
        series_id: Some(weekend.series_id as i64),
        series_name: None,
        season_name: None,
        season_year: None,
        season_quarter: None,
        track_id: Some(weekend.track_id),
        track_name: Some(weekend.track_display_name.clone()),
        track_config: if weekend.track_config_name.is_empty() {
            None
        } else {
            Some(weekend.track_config_name.clone())
        },
        event_type: None,
        event_type_name: None,
        start_time: now,
        end_time: None,
        weather_summary: None,
        sof: None,
        cars_json: None,
        raw_json: String::new(),
        source: "live".to_string(),
        captured_at: None,
    };

    let segment_row = SegmentRow {
        sub_session_id,
        simsession_type: simsession_type.clone(),
        simsession_num: snapshot.session_num,
        event_type_name: Some(session_type.clone()),
        start_time: now,
        end_time: Some(now),
        weather_summary: weekend.track_weather_type.clone(),
        sof,
        raw_json: Some(serde_json::to_string(snapshot)?),
        source: "live".to_string(),
        captured_at: Some(now),
        is_finalized,
    };

    let player_car_idx = yaml.driver_info.driver_car_idx;

    let result_rows: Vec<ResultRow> = snapshot
        .entries
        .iter()
        .filter(|e| {
            !driver_map
                .get(&e.car_idx)
                .map(|d| d.car_is_pace_car != 0)
                .unwrap_or(false)
        })
        .map(|entry| {
            build_result_row(
                sub_session_id,
                &simsession_type,
                entry,
                driver_map.get(&entry.car_idx).copied(),
                results_by_car.get(&entry.car_idx).copied(),
                entry.car_idx == player_car_idx,
            )
        })
        .collect();

    Ok(LiveCapture { session_row, segment_row, result_rows, sub_session_id })
}

fn build_result_row(
    sub_session_id: i64,
    simsession_type: &str,
    entry: &StandingEntry,
    driver: Option<&DriverEntry>,
    result_pos: Option<&ResultPosition>,
    is_player: bool,
) -> ResultRow {
    let simsession_type = simsession_type.to_string();

    let best_lap_ms = if entry.best_lap_time > 0.0 {
        Some((entry.best_lap_time * 1000.0) as i32)
    } else {
        None
    };

    // Prefer YAML ResultsPositions for incidents (authoritative); -1 = not available
    let incidents = result_pos.and_then(|rp| {
        if rp.incidents >= 0 { Some(rp.incidents) } else { None }
    });

    // YAML laps_complete is more accurate than the live lap counter
    let laps_complete = result_pos
        .map(|rp| rp.laps_complete)
        .or_else(|| Some(entry.lap));

    let reason_out = result_pos.and_then(|rp| {
        if rp.reason_out_str.is_empty() || rp.reason_out_str == "Running" {
            None
        } else {
            Some(rp.reason_out_str.clone())
        }
    });

    // Positions are stored 1-based (1 = winner / class leader).
    // Both iRacing YAML ResultsPositions.Position and SDK CarIdxPosition are 1-based.
    let finish_position = result_pos
        .map(|rp| rp.position)
        .or_else(|| Some(entry.position));
    let class_position = result_pos
        .map(|rp| rp.class_position)
        .or_else(|| Some(entry.class_position));

    let last_sectors_json = if entry.last_sector_times.is_empty() {
        None
    } else {
        serde_json::to_string(&entry.last_sector_times).ok()
    };

    let best_sectors_json = if entry.best_sector_times.is_empty() {
        None
    } else {
        serde_json::to_string(&entry.best_sector_times).ok()
    };

    let car_name = driver.and_then(|d| d.car_screen_name_short.clone());

    ResultRow {
        sub_session_id,
        simsession_type,
        car_idx: entry.car_idx,
        cust_id: None, // CustID not available in live SDK YAML
        display_name: Some(entry.user_name.clone()),
        finish_position,
        starting_position: None, // not available from live SDK
        class_position,
        laps_complete,
        incidents,
        best_lap_ms,
        average_lap_ms: None,
        oldi_rating: Some(entry.irating),
        newi_rating: Some(entry.irating), // no pre-race delta available from live SDK
        oldsr: None,
        newsr: None,
        car_id: None,
        car_name,
        car_class_id: Some(entry.car_class_id as i64),
        car_class_name: Some(entry.car_class_short_name.clone()),
        is_player,
        last_sectors_json,
        best_sectors_json,
        pit_stops: Some(entry.pit_stops as i32),
        tire_compound: entry.tire_compound,
        car_number: Some(entry.car_number.clone()),
        safety_rating: Some(entry.safety_rating.clone()),
        lic_color: entry.lic_color.map(|v| v as i32),
        car_class_color: entry.car_class_color.map(|v| v as i32),
        reason_out,
        source: "live".to_string(),
    }
}

fn session_type_to_label(t: &str) -> &'static str {
    match t {
        "Practice" => "P",
        "Lone Qualify" | "Open Qualify" | "Qualify" => "Q",
        _ => "R", // Race, Offline Testing, unknown → Race
    }
}

fn compute_sof(iratings: &[i32]) -> i32 {
    let n = iratings.len() as f64;
    if n == 0.0 {
        return 0;
    }
    let sum: f64 = iratings
        .iter()
        .map(|&ir| 2f64.powf(-(ir as f64) / 1600.0))
        .sum();
    let br = 1600.0 / f64::ln(2.0);
    (br * f64::ln(n / sum)).round() as i32
}

/// Async task: receives live captures from the SDK loop and writes them to SQLite.
pub async fn run(
    db: Db,
    push_tx: broadcast::Sender<ServerMessage>,
    mut rx: mpsc::Receiver<LiveCapture>,
) {
    while let Some(capture) = rx.recv().await {
        let sub_id = capture.sub_session_id;
        let is_finalized = capture.segment_row.is_finalized;
        let session = capture.session_row;
        let segment = capture.segment_row;
        let results = capture.result_rows;

        let res = db.with(move |c| {
            upsert_subsession(c, &session)?;
            upsert_segment(c, &segment)?;
            insert_result_rows(c, &results)
        }).await;
        match res {
            Ok(()) => {
                if is_finalized {
                    log::info!("live_capture: stored subsession {sub_id}");
                } else {
                    log::debug!("live_capture: live-persisted subsession {sub_id}");
                }
                let _ = push_tx.send(ServerMessage::ResultsUpdated { sub_session_id: sub_id });
            }
            Err(e) => log::warn!("live_capture: db write failed for {sub_id}: {e}"),
        }
    }
}

/// Write a batch of lap rows (called from the lap buffer on flush).
pub async fn write_laps(db: &Db, laps: Vec<LapRow>) {
    if laps.is_empty() {
        return;
    }
    if let Err(e) = db
        .with(move |c| crate::persistence::queries::insert_laps(c, &laps))
        .await
    {
        log::warn!("live_capture: lap write failed: {e}");
    }
}
