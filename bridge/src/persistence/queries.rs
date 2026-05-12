//! Database read/write helpers for race results.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ─── DTOs used by both bridge (write) and handler (read/send) ─────────────

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    #[ts(type = "number")]
    pub sub_session_id: i64,
    pub series_name: Option<String>,
    pub season_name: Option<String>,
    pub track_name: Option<String>,
    pub track_config: Option<String>,
    pub event_type_name: Option<String>,
    #[ts(type = "number")]
    pub start_time: i64,
    #[ts(type = "number | null")]
    pub end_time: Option<i64>,
    pub sof: Option<i32>,
    pub player_finish_position: Option<i32>,
    pub player_incidents: Option<i32>,
    pub player_oldi_rating: Option<i32>,
    pub player_newi_rating: Option<i32>,
    pub player_oldsr: Option<i32>,
    pub player_newsr: Option<i32>,
    pub player_car_name: Option<String>,
    pub player_best_lap_ms: Option<i32>,
    pub player_laps_complete: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub summary: SessionSummary,
    pub results: Vec<DriverResult>,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct DriverResult {
    pub simsession_type: String,
    #[ts(type = "number")]
    pub cust_id: i64,
    pub display_name: Option<String>,
    pub finish_position: Option<i32>,
    pub starting_position: Option<i32>,
    pub laps_complete: Option<i32>,
    pub incidents: Option<i32>,
    pub best_lap_ms: Option<i32>,
    pub average_lap_ms: Option<i32>,
    pub oldi_rating: Option<i32>,
    pub newi_rating: Option<i32>,
    pub oldsr: Option<i32>,
    pub newsr: Option<i32>,
    pub car_name: Option<String>,
    pub car_class_name: Option<String>,
    pub is_player: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct ResultsFilter {
    #[ts(type = "number | null")]
    pub track_id: Option<i64>,
    #[ts(type = "number | null")]
    pub car_id: Option<i64>,
    #[ts(type = "number | null")]
    pub series_id: Option<i64>,
    pub event_type: Option<i32>,
    #[ts(type = "number | null")]
    pub date_from: Option<i64>,
    #[ts(type = "number | null")]
    pub date_to: Option<i64>,
    #[ts(type = "number | null")]
    pub limit: Option<i64>,
    #[ts(type = "number | null")]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct FilterOptions {
    pub tracks: Vec<TrackOption>,
    pub cars: Vec<CarOption>,
    pub series: Vec<SeriesOption>,
    pub event_types: Vec<EventTypeOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct TrackOption {
    #[ts(type = "number")]
    pub track_id: i64,
    pub track_name: String,
    pub track_config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct CarOption {
    #[ts(type = "number")]
    pub car_id: i64,
    pub car_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct SeriesOption {
    #[ts(type = "number")]
    pub series_id: i64,
    pub series_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct EventTypeOption {
    pub event_type: i32,
    pub event_type_name: String,
}

// ─── Write helpers ────────────────────────────────────────────────────────

pub struct SessionRow {
    pub sub_session_id: i64,
    pub session_id: Option<i64>,
    pub series_id: Option<i64>,
    pub series_name: Option<String>,
    pub season_name: Option<String>,
    pub season_year: Option<i32>,
    pub season_quarter: Option<i32>,
    pub track_id: Option<i64>,
    pub track_name: Option<String>,
    pub track_config: Option<String>,
    pub event_type: Option<i32>,
    pub event_type_name: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub weather_summary: Option<String>,
    pub sof: Option<i32>,
    pub cars_json: Option<String>,
    pub raw_json: String,
}

pub struct ResultRow {
    pub sub_session_id: i64,
    pub simsession_type: String,
    pub cust_id: i64,
    pub display_name: Option<String>,
    pub finish_position: Option<i32>,
    pub starting_position: Option<i32>,
    pub laps_complete: Option<i32>,
    pub incidents: Option<i32>,
    pub best_lap_ms: Option<i32>,
    pub average_lap_ms: Option<i32>,
    pub oldi_rating: Option<i32>,
    pub newi_rating: Option<i32>,
    pub oldsr: Option<i32>,
    pub newsr: Option<i32>,
    pub car_id: Option<i64>,
    pub car_name: Option<String>,
    pub car_class_id: Option<i64>,
    pub car_class_name: Option<String>,
    pub is_player: bool,
}

pub fn insert_session(conn: &Connection, session: &SessionRow, results: &[ResultRow]) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO sessions
         (sub_session_id, session_id, series_id, series_name, season_name, season_year,
          season_quarter, track_id, track_name, track_config, event_type, event_type_name,
          start_time, end_time, weather_summary, sof, cars_json, raw_json)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        params![
            session.sub_session_id,
            session.session_id,
            session.series_id,
            session.series_name,
            session.season_name,
            session.season_year,
            session.season_quarter,
            session.track_id,
            session.track_name,
            session.track_config,
            session.event_type,
            session.event_type_name,
            session.start_time,
            session.end_time,
            session.weather_summary,
            session.sof,
            session.cars_json,
            session.raw_json,
        ],
    )
    .context("insert session")?;

    for r in results {
        conn.execute(
            "INSERT OR REPLACE INTO session_results
             (sub_session_id, simsession_type, cust_id, display_name, finish_position,
              starting_position, laps_complete, incidents, best_lap_ms, average_lap_ms,
              oldi_rating, newi_rating, oldsr, newsr, car_id, car_name,
              car_class_id, car_class_name, is_player)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
            params![
                r.sub_session_id,
                r.simsession_type,
                r.cust_id,
                r.display_name,
                r.finish_position,
                r.starting_position,
                r.laps_complete,
                r.incidents,
                r.best_lap_ms,
                r.average_lap_ms,
                r.oldi_rating,
                r.newi_rating,
                r.oldsr,
                r.newsr,
                r.car_id,
                r.car_name,
                r.car_class_id,
                r.car_class_name,
                r.is_player as i32,
            ],
        )
        .context("insert result row")?;
    }
    Ok(())
}

// ─── Read helpers ─────────────────────────────────────────────────────────

pub fn query_sessions(conn: &Connection, filter: &ResultsFilter) -> Result<(Vec<SessionSummary>, i64)> {
    let limit = filter.limit.unwrap_or(50).min(200);
    let offset = filter.offset.unwrap_or(0);

    // Build a WHERE clause dynamically.
    let mut where_parts: Vec<String> = Vec::new();
    if filter.track_id.is_some() { where_parts.push("s.track_id = ?".into()); }
    if filter.series_id.is_some() { where_parts.push("s.series_id = ?".into()); }
    if filter.event_type.is_some() { where_parts.push("s.event_type = ?".into()); }
    if filter.date_from.is_some() { where_parts.push("s.start_time >= ?".into()); }
    if filter.date_to.is_some() { where_parts.push("s.start_time <= ?".into()); }
    if filter.car_id.is_some() {
        where_parts.push("EXISTS (SELECT 1 FROM session_results sr WHERE sr.sub_session_id = s.sub_session_id AND sr.car_id = ? AND sr.is_player = 1)".into());
    }

    let where_sql = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

    // Build param list in order of where_parts.
    // We use a helper to avoid raw SQL injection — all values are typed parameters.
    let mut count_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut list_params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    macro_rules! push_param {
        ($val:expr) => {{
            let v = $val.clone();
            let v2 = $val.clone();
            count_params.push(Box::new(v));
            list_params.push(Box::new(v2));
        }};
    }

    if let Some(v) = filter.track_id { push_param!(v); }
    if let Some(v) = filter.series_id { push_param!(v); }
    if let Some(v) = filter.event_type { push_param!(v as i64); }
    if let Some(v) = filter.date_from { push_param!(v); }
    if let Some(v) = filter.date_to { push_param!(v); }
    if let Some(v) = filter.car_id { push_param!(v); }

    let count_sql = format!("SELECT COUNT(*) FROM sessions s {where_sql}");
    let total: i64 = {
        let refs: Vec<&dyn rusqlite::ToSql> = count_params.iter().map(|b| b.as_ref()).collect();
        conn.query_row(&count_sql, refs.as_slice(), |r| r.get(0))?
    };

    let list_sql = format!(
        "SELECT s.sub_session_id, s.series_name, s.season_name, s.track_name, s.track_config,
                s.event_type_name, s.start_time, s.end_time, s.sof,
                r.finish_position, r.incidents, r.oldi_rating, r.newi_rating,
                r.oldsr, r.newsr, r.car_name, r.best_lap_ms, r.laps_complete
         FROM sessions s
         LEFT JOIN session_results r ON r.sub_session_id = s.sub_session_id
             AND r.is_player = 1 AND r.simsession_type = 'R'
         {where_sql}
         ORDER BY s.start_time DESC
         LIMIT {limit} OFFSET {offset}"
    );

    let mut sessions = Vec::new();
    {
        let refs: Vec<&dyn rusqlite::ToSql> = list_params.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&list_sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok(SessionSummary {
                sub_session_id: row.get(0)?,
                series_name: row.get(1)?,
                season_name: row.get(2)?,
                track_name: row.get(3)?,
                track_config: row.get(4)?,
                event_type_name: row.get(5)?,
                start_time: row.get(6)?,
                end_time: row.get(7)?,
                sof: row.get(8)?,
                player_finish_position: row.get(9)?,
                player_incidents: row.get(10)?,
                player_oldi_rating: row.get(11)?,
                player_newi_rating: row.get(12)?,
                player_oldsr: row.get(13)?,
                player_newsr: row.get(14)?,
                player_car_name: row.get(15)?,
                player_best_lap_ms: row.get(16)?,
                player_laps_complete: row.get(17)?,
            })
        })?;
        for r in rows { sessions.push(r?); }
    }

    Ok((sessions, total))
}

pub fn get_session_detail(conn: &Connection, sub_session_id: i64) -> Result<Option<SessionDetail>> {
    let summary = {
        let mut stmt = conn.prepare(
            "SELECT s.sub_session_id, s.series_name, s.season_name, s.track_name, s.track_config,
                    s.event_type_name, s.start_time, s.end_time, s.sof,
                    r.finish_position, r.incidents, r.oldi_rating, r.newi_rating,
                    r.oldsr, r.newsr, r.car_name, r.best_lap_ms, r.laps_complete
             FROM sessions s
             LEFT JOIN session_results r ON r.sub_session_id = s.sub_session_id
                 AND r.is_player = 1 AND r.simsession_type = 'R'
             WHERE s.sub_session_id = ?1",
        )?;
        let mut rows = stmt.query_map([sub_session_id], |row| {
            Ok(SessionSummary {
                sub_session_id: row.get(0)?,
                series_name: row.get(1)?,
                season_name: row.get(2)?,
                track_name: row.get(3)?,
                track_config: row.get(4)?,
                event_type_name: row.get(5)?,
                start_time: row.get(6)?,
                end_time: row.get(7)?,
                sof: row.get(8)?,
                player_finish_position: row.get(9)?,
                player_incidents: row.get(10)?,
                player_oldi_rating: row.get(11)?,
                player_newi_rating: row.get(12)?,
                player_oldsr: row.get(13)?,
                player_newsr: row.get(14)?,
                player_car_name: row.get(15)?,
                player_best_lap_ms: row.get(16)?,
                player_laps_complete: row.get(17)?,
            })
        })?;
        match rows.next() {
            Some(r) => r?,
            None => return Ok(None),
        }
    };

    let raw_json: String = conn.query_row(
        "SELECT raw_json FROM sessions WHERE sub_session_id = ?1",
        [sub_session_id],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT simsession_type, cust_id, display_name, finish_position, starting_position,
                laps_complete, incidents, best_lap_ms, average_lap_ms,
                oldi_rating, newi_rating, oldsr, newsr,
                car_name, car_class_name, is_player
         FROM session_results WHERE sub_session_id = ?1
         ORDER BY simsession_type, finish_position",
    )?;
    let results: Vec<DriverResult> = stmt
        .query_map([sub_session_id], |row| {
            Ok(DriverResult {
                simsession_type: row.get(0)?,
                cust_id: row.get(1)?,
                display_name: row.get(2)?,
                finish_position: row.get(3)?,
                starting_position: row.get(4)?,
                laps_complete: row.get(5)?,
                incidents: row.get(6)?,
                best_lap_ms: row.get(7)?,
                average_lap_ms: row.get(8)?,
                oldi_rating: row.get(9)?,
                newi_rating: row.get(10)?,
                oldsr: row.get(11)?,
                newsr: row.get(12)?,
                car_name: row.get(13)?,
                car_class_name: row.get(14)?,
                is_player: {
                    let v: i32 = row.get(15)?;
                    v != 0
                },
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    Ok(Some(SessionDetail { summary, results, raw_json }))
}

pub fn get_filter_options(conn: &Connection) -> Result<FilterOptions> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT track_id, track_name, track_config FROM sessions WHERE track_id IS NOT NULL ORDER BY track_name",
    )?;
    let tracks: Vec<TrackOption> = stmt
        .query_map([], |r| {
            Ok(TrackOption { track_id: r.get(0)?, track_name: r.get(1)?, track_config: r.get(2)? })
        })?
        .collect::<std::result::Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT sr.car_id, sr.car_name FROM session_results sr WHERE sr.car_id IS NOT NULL AND sr.is_player = 1 ORDER BY sr.car_name",
    )?;
    let cars: Vec<CarOption> = stmt
        .query_map([], |r| Ok(CarOption { car_id: r.get(0)?, car_name: r.get(1)? }))?
        .collect::<std::result::Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT series_id, series_name FROM sessions WHERE series_id IS NOT NULL ORDER BY series_name",
    )?;
    let series: Vec<SeriesOption> = stmt
        .query_map([], |r| Ok(SeriesOption { series_id: r.get(0)?, series_name: r.get(1)? }))?
        .collect::<std::result::Result<_, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT event_type, event_type_name FROM sessions WHERE event_type IS NOT NULL ORDER BY event_type",
    )?;
    let event_types: Vec<EventTypeOption> = stmt
        .query_map([], |r| Ok(EventTypeOption { event_type: r.get(0)?, event_type_name: r.get(1)? }))?
        .collect::<std::result::Result<_, _>>()?;

    Ok(FilterOptions { tracks, cars, series, event_types })
}

// ─── Pending-fetch queue ───────────────────────────────────────────────────

pub fn enqueue_pending(conn: &Connection, sub_session_id: i64, next_retry_at: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO pending_fetches (sub_session_id, next_retry_at, attempts) VALUES (?1, ?2, 0)",
        params![sub_session_id, next_retry_at],
    )?;
    Ok(())
}

pub fn get_due_pending(conn: &Connection, now: i64) -> Result<Vec<(i64, i32)>> {
    let mut stmt = conn.prepare(
        "SELECT sub_session_id, attempts FROM pending_fetches WHERE next_retry_at <= ?1 ORDER BY next_retry_at LIMIT 10",
    )?;
    let rows: Vec<(i64, i32)> = stmt
        .query_map([now], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<_, _>>()?;
    Ok(rows)
}

pub fn update_pending_retry(conn: &Connection, sub_session_id: i64, next_retry_at: i64, attempts: i32, last_error: &str) -> Result<()> {
    conn.execute(
        "UPDATE pending_fetches SET next_retry_at = ?1, attempts = ?2, last_error = ?3 WHERE sub_session_id = ?4",
        params![next_retry_at, attempts, last_error, sub_session_id],
    )?;
    Ok(())
}

pub fn remove_pending(conn: &Connection, sub_session_id: i64) -> Result<()> {
    conn.execute("DELETE FROM pending_fetches WHERE sub_session_id = ?1", [sub_session_id])?;
    Ok(())
}

pub fn session_exists(conn: &Connection, sub_session_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE sub_session_id = ?1",
        [sub_session_id],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}
