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
    pub simsession_type: String,
    pub is_finalized: bool,
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
    pub source: Option<String>,
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
    pub car_idx: i32,
    #[ts(type = "number | null")]
    pub cust_id: Option<i64>,
    pub class_position: Option<i32>,
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
    // Live-capture fields — None for API-sourced rows
    pub last_sectors_json: Option<String>,
    pub best_sectors_json: Option<String>,
    pub pit_stops: Option<i32>,
    pub car_number: Option<String>,
    pub safety_rating: Option<String>,
    pub lic_color: Option<i32>,
    pub car_class_color: Option<i32>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct LapRow {
    #[ts(type = "number")]
    pub sub_session_id: i64,
    pub simsession_num: i32,
    pub car_idx: i32,
    pub lap_num: i32,
    #[ts(type = "number | null")]
    pub cust_id: Option<i64>,
    pub lap_time_sec: Option<f32>,
    pub sectors_json: Option<String>,
    pub valid: bool,
    pub in_lap: bool,
    pub out_lap: bool,
    pub incidents_delta: Option<i32>,
    pub position_at_end: Option<i32>,
    pub air_temp: Option<f32>,
    pub track_temp: Option<f32>,
    pub track_wetness: Option<i32>,
    pub rubber_state: Option<i32>,
    pub session_time: Option<f64>,
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

pub struct SegmentRow {
    pub sub_session_id: i64,
    pub simsession_type: String,
    pub simsession_num: i32,
    pub event_type_name: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub weather_summary: Option<String>,
    pub sof: Option<i32>,
    pub raw_json: Option<String>,
    pub source: String,
    pub captured_at: Option<i64>,
    pub is_finalized: bool,
}

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
    pub source: String,            // 'live' or 'api'
    pub captured_at: Option<i64>,  // Unix timestamp in seconds, live only
}

pub struct ResultRow {
    pub sub_session_id: i64,
    pub simsession_type: String,
    pub car_idx: i32,              // real car_idx for live data; -1 sentinel for API-sourced rows
    pub cust_id: Option<i64>,      // None for AI cars
    pub display_name: Option<String>,
    pub finish_position: Option<i32>,
    pub starting_position: Option<i32>,
    pub class_position: Option<i32>,
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
    pub last_sectors_json: Option<String>,
    pub best_sectors_json: Option<String>,
    pub pit_stops: Option<i32>,
    pub tire_compound: Option<i32>,
    pub car_number: Option<String>,
    pub safety_rating: Option<String>,
    pub lic_color: Option<i32>,
    pub car_class_color: Option<i32>,
    pub reason_out: Option<String>,
    pub source: String,            // 'live' or 'api'
}

/// Insert or ignore the subsession-global row. For NULL fields, fills them in if better data
/// arrives later (e.g. API after live capture), but never overwrites existing non-NULL values.
pub fn upsert_subsession(conn: &Connection, session: &SessionRow) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sessions
         (sub_session_id, session_id, series_id, series_name, season_name, season_year,
          season_quarter, track_id, track_name, track_config, event_type, cars_json,
          start_time, source, raw_json)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
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
            session.cars_json,
            session.start_time,
            session.source,
            session.raw_json,
        ],
    )
    .context("insert subsession")?;
    // Fill in NULL columns only — never overwrite existing data.
    conn.execute(
        "UPDATE sessions SET
         session_id     = COALESCE(session_id,     ?2),
         series_id      = COALESCE(series_id,      ?3),
         series_name    = COALESCE(series_name,    ?4),
         season_name    = COALESCE(season_name,    ?5),
         season_year    = COALESCE(season_year,    ?6),
         season_quarter = COALESCE(season_quarter, ?7),
         track_id       = COALESCE(track_id,       ?8),
         track_name     = COALESCE(track_name,     ?9),
         track_config   = COALESCE(track_config,   ?10),
         event_type     = COALESCE(event_type,     ?11),
         event_type_name= COALESCE(event_type_name,?13),
         cars_json      = COALESCE(cars_json,      ?12),
         weather_summary= COALESCE(weather_summary,?14),
         sof            = COALESCE(sof,            ?15)
         WHERE sub_session_id = ?1",
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
            session.cars_json,
            session.event_type_name,
            session.weather_summary,
            session.sof,
        ],
    )
    .context("update subsession nulls")?;
    Ok(())
}

/// Upsert a per-phase segment row.
/// Once finalized, raw_json/end_time/captured_at are frozen — *unless* the update
/// arrives within 30 s of the last capture (grace window for late ResultsPositions).
pub fn upsert_segment(conn: &Connection, seg: &SegmentRow) -> Result<()> {
    conn.execute(
        "INSERT INTO session_segments
         (sub_session_id, simsession_type, simsession_num, event_type_name,
          start_time, end_time, weather_summary, sof, raw_json, source, captured_at, is_finalized)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(sub_session_id, simsession_type) DO UPDATE SET
           simsession_num  = excluded.simsession_num,
           event_type_name = COALESCE(excluded.event_type_name, event_type_name),
           end_time        = CASE WHEN is_finalized = 0
                                    OR (captured_at IS NOT NULL AND excluded.captured_at - captured_at < 30000)
                                  THEN excluded.end_time    ELSE end_time    END,
           weather_summary = COALESCE(excluded.weather_summary, weather_summary),
           sof             = COALESCE(excluded.sof, sof),
           raw_json        = CASE WHEN is_finalized = 0
                                    OR (captured_at IS NOT NULL AND excluded.captured_at - captured_at < 30000)
                                  THEN excluded.raw_json    ELSE raw_json    END,
           source          = excluded.source,
           captured_at     = CASE WHEN is_finalized = 0
                                    OR (captured_at IS NOT NULL AND excluded.captured_at - captured_at < 30000)
                                  THEN excluded.captured_at ELSE captured_at END,
           is_finalized    = MAX(is_finalized, excluded.is_finalized)",
        params![
            seg.sub_session_id,
            seg.simsession_type,
            seg.simsession_num,
            seg.event_type_name,
            seg.start_time,
            seg.end_time,
            seg.weather_summary,
            seg.sof,
            seg.raw_json,
            seg.source,
            seg.captured_at,
            seg.is_finalized as i32,
        ],
    )
    .context("upsert segment")?;
    Ok(())
}

pub fn insert_result_rows(conn: &Connection, results: &[ResultRow]) -> Result<()> {
    for r in results {
        conn.execute(
            "INSERT OR REPLACE INTO session_results
             (sub_session_id, simsession_type, car_idx, cust_id, display_name,
              finish_position, starting_position, class_position,
              laps_complete, incidents, best_lap_ms, average_lap_ms,
              oldi_rating, newi_rating, oldsr, newsr,
              car_id, car_name, car_class_id, car_class_name, is_player, source,
              last_sectors_json, best_sectors_json, pit_stops, tire_compound,
              car_number, safety_rating, lic_color, car_class_color, reason_out)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                     ?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31)",
            params![
                r.sub_session_id,
                r.simsession_type,
                r.car_idx,
                r.cust_id,
                r.display_name,
                r.finish_position,
                r.starting_position,
                r.class_position,
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
                r.source,
                r.last_sectors_json,
                r.best_sectors_json,
                r.pit_stops,
                r.tire_compound,
                r.car_number,
                r.safety_rating,
                r.lic_color,
                r.car_class_color,
                r.reason_out,
            ],
        )
        .context("insert result row")?;
    }
    Ok(())
}

pub fn insert_laps(conn: &Connection, laps: &[LapRow]) -> Result<()> {
    for lap in laps {
        conn.execute(
            "INSERT OR REPLACE INTO live_session_laps
             (sub_session_id, simsession_num, car_idx, lap_num, cust_id,
              lap_time_sec, sectors_json, valid, in_lap, out_lap,
              incidents_delta, position_at_end, air_temp, track_temp,
              track_wetness, rubber_state, session_time)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![
                lap.sub_session_id,
                lap.simsession_num,
                lap.car_idx,
                lap.lap_num,
                lap.cust_id,
                lap.lap_time_sec,
                lap.sectors_json,
                lap.valid as i32,
                lap.in_lap as i32,
                lap.out_lap as i32,
                lap.incidents_delta,
                lap.position_at_end,
                lap.air_temp,
                lap.track_temp,
                lap.track_wetness,
                lap.rubber_state,
                lap.session_time,
            ],
        )
        .context("insert lap row")?;
    }
    Ok(())
}

// ─── Read helpers ─────────────────────────────────────────────────────────

pub fn query_sessions(conn: &Connection, filter: &ResultsFilter) -> Result<(Vec<SessionSummary>, i64)> {
    let limit = filter.limit.unwrap_or(50).min(200);
    let offset = filter.offset.unwrap_or(0);

    let mut where_parts: Vec<String> = Vec::new();
    if filter.track_id.is_some() { where_parts.push("s.track_id = ?".into()); }
    if filter.series_id.is_some() { where_parts.push("s.series_id = ?".into()); }
    if filter.event_type.is_some() { where_parts.push("s.event_type = ?".into()); }
    if filter.date_from.is_some() { where_parts.push("sg.start_time >= ?".into()); }
    if filter.date_to.is_some() { where_parts.push("sg.start_time <= ?".into()); }
    if filter.car_id.is_some() {
        where_parts.push("EXISTS (SELECT 1 FROM session_results sr WHERE sr.sub_session_id = s.sub_session_id AND sr.car_id = ? AND sr.is_player = 1)".into());
    }

    let where_sql = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

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

    // primary_seg picks the highest-priority phase per sub_session_id (R > Q > P > ?).
    let primary_seg_cte = "WITH primary_seg AS (
         SELECT sg.*,
                ROW_NUMBER() OVER (
                    PARTITION BY sg.sub_session_id
                    ORDER BY CASE sg.simsession_type WHEN 'R' THEN 0 WHEN 'Q' THEN 1 WHEN 'P' THEN 2 ELSE 3 END,
                             sg.start_time DESC
                ) AS rn
         FROM session_segments sg
     )";

    let count_sql = format!(
        "{primary_seg_cte}
         SELECT COUNT(*) FROM sessions s
         JOIN primary_seg sg ON sg.sub_session_id = s.sub_session_id AND sg.rn = 1
         {where_sql}"
    );
    let total: i64 = {
        let refs: Vec<&dyn rusqlite::ToSql> = count_params.iter().map(|b| b.as_ref()).collect();
        conn.query_row(&count_sql, refs.as_slice(), |r| r.get(0))?
    };

    let list_sql = format!(
        "{primary_seg_cte}
         SELECT s.sub_session_id, sg.simsession_type, sg.is_finalized,
                s.series_name, s.season_name, s.track_name, s.track_config,
                sg.event_type_name, sg.start_time, sg.end_time, sg.sof, sg.source,
                r.finish_position, r.incidents, r.oldi_rating, r.newi_rating,
                r.oldsr, r.newsr, r.car_name, r.best_lap_ms, r.laps_complete
         FROM sessions s
         JOIN primary_seg sg ON sg.sub_session_id = s.sub_session_id AND sg.rn = 1
         LEFT JOIN session_results r ON r.sub_session_id = s.sub_session_id
             AND r.is_player = 1 AND r.simsession_type = sg.simsession_type
         {where_sql}
         ORDER BY sg.start_time DESC
         LIMIT {limit} OFFSET {offset}"
    );

    let mut sessions = Vec::new();
    {
        let refs: Vec<&dyn rusqlite::ToSql> = list_params.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&list_sql)?;
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok(SessionSummary {
                sub_session_id: row.get(0)?,
                simsession_type: row.get(1)?,
                is_finalized: { let v: i32 = row.get(2)?; v != 0 },
                series_name: row.get(3)?,
                season_name: row.get(4)?,
                track_name: row.get(5)?,
                track_config: row.get(6)?,
                event_type_name: row.get(7)?,
                start_time: row.get(8)?,
                end_time: row.get(9)?,
                sof: row.get(10)?,
                source: row.get(11)?,
                player_finish_position: row.get(12)?,
                player_incidents: row.get(13)?,
                player_oldi_rating: row.get(14)?,
                player_newi_rating: row.get(15)?,
                player_oldsr: row.get(16)?,
                player_newsr: row.get(17)?,
                player_car_name: row.get(18)?,
                player_best_lap_ms: row.get(19)?,
                player_laps_complete: row.get(20)?,
            })
        })?;
        for r in rows { sessions.push(r?); }
    }

    Ok((sessions, total))
}

pub fn get_session_detail(conn: &Connection, sub_session_id: i64) -> Result<Option<SessionDetail>> {
    // Pick the "primary" segment for the summary header: R > Q > P, then most recent.
    let summary = {
        let mut stmt = conn.prepare(
            "SELECT s.sub_session_id, sg.simsession_type, sg.is_finalized,
                    s.series_name, s.season_name, s.track_name, s.track_config,
                    sg.event_type_name, sg.start_time, sg.end_time, sg.sof, sg.source,
                    r.finish_position, r.incidents, r.oldi_rating, r.newi_rating,
                    r.oldsr, r.newsr, r.car_name, r.best_lap_ms, r.laps_complete
             FROM sessions s
             JOIN session_segments sg ON sg.sub_session_id = s.sub_session_id
             LEFT JOIN session_results r ON r.sub_session_id = s.sub_session_id
                 AND r.is_player = 1 AND r.simsession_type = sg.simsession_type
             WHERE s.sub_session_id = ?1
             ORDER BY CASE sg.simsession_type WHEN 'R' THEN 0 WHEN 'Q' THEN 1 ELSE 2 END
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map([sub_session_id], |row| {
            Ok(SessionSummary {
                sub_session_id: row.get(0)?,
                simsession_type: row.get(1)?,
                is_finalized: { let v: i32 = row.get(2)?; v != 0 },
                series_name: row.get(3)?,
                season_name: row.get(4)?,
                track_name: row.get(5)?,
                track_config: row.get(6)?,
                event_type_name: row.get(7)?,
                start_time: row.get(8)?,
                end_time: row.get(9)?,
                sof: row.get(10)?,
                source: row.get(11)?,
                player_finish_position: row.get(12)?,
                player_incidents: row.get(13)?,
                player_oldi_rating: row.get(14)?,
                player_newi_rating: row.get(15)?,
                player_oldsr: row.get(16)?,
                player_newsr: row.get(17)?,
                player_car_name: row.get(18)?,
                player_best_lap_ms: row.get(19)?,
                player_laps_complete: row.get(20)?,
            })
        })?;
        match rows.next() {
            Some(r) => r?,
            None => return Ok(None),
        }
    };

    let raw_json: String = conn.query_row(
        "SELECT COALESCE(raw_json, '')
         FROM session_segments WHERE sub_session_id = ?1
         ORDER BY CASE simsession_type WHEN 'R' THEN 0 WHEN 'Q' THEN 1 ELSE 2 END
         LIMIT 1",
        [sub_session_id],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT simsession_type, car_idx, cust_id, class_position,
                display_name, finish_position, starting_position,
                laps_complete, incidents, best_lap_ms, average_lap_ms,
                oldi_rating, newi_rating, oldsr, newsr,
                car_name, car_class_name, is_player,
                last_sectors_json, best_sectors_json, pit_stops,
                car_number, safety_rating, lic_color, car_class_color, source
         FROM session_results WHERE sub_session_id = ?1
         ORDER BY simsession_type, COALESCE(finish_position, 9999)",
    )?;
    let results: Vec<DriverResult> = stmt
        .query_map([sub_session_id], |row| {
            Ok(DriverResult {
                simsession_type: row.get(0)?,
                car_idx: row.get(1)?,
                cust_id: row.get(2)?,
                class_position: row.get(3)?,
                display_name: row.get(4)?,
                finish_position: row.get(5)?,
                starting_position: row.get(6)?,
                laps_complete: row.get(7)?,
                incidents: row.get(8)?,
                best_lap_ms: row.get(9)?,
                average_lap_ms: row.get(10)?,
                oldi_rating: row.get(11)?,
                newi_rating: row.get(12)?,
                oldsr: row.get(13)?,
                newsr: row.get(14)?,
                car_name: row.get(15)?,
                car_class_name: row.get(16)?,
                is_player: {
                    let v: i32 = row.get(17)?;
                    v != 0
                },
                last_sectors_json: row.get(18)?,
                best_sectors_json: row.get(19)?,
                pit_stops: row.get(20)?,
                car_number: row.get(21)?,
                safety_rating: row.get(22)?,
                lic_color: row.get(23)?,
                car_class_color: row.get(24)?,
                source: row.get(25)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;

    Ok(Some(SessionDetail { summary, results, raw_json }))
}

pub fn get_laps(conn: &Connection, sub_session_id: i64, car_idx: Option<i32>) -> Result<Vec<LapRow>> {
    fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LapRow> {
        Ok(LapRow {
            sub_session_id: row.get(0)?,
            simsession_num: row.get(1)?,
            car_idx: row.get(2)?,
            lap_num: row.get(3)?,
            cust_id: row.get(4)?,
            lap_time_sec: row.get(5)?,
            sectors_json: row.get(6)?,
            valid: { let v: i32 = row.get(7)?; v != 0 },
            in_lap: { let v: i32 = row.get(8)?; v != 0 },
            out_lap: { let v: i32 = row.get(9)?; v != 0 },
            incidents_delta: row.get(10)?,
            position_at_end: row.get(11)?,
            air_temp: row.get(12)?,
            track_temp: row.get(13)?,
            track_wetness: row.get(14)?,
            rubber_state: row.get(15)?,
            session_time: row.get(16)?,
        })
    }

    const SELECT: &str =
        "SELECT sub_session_id, simsession_num, car_idx, lap_num, cust_id,
                lap_time_sec, sectors_json, valid, in_lap, out_lap,
                incidents_delta, position_at_end, air_temp, track_temp,
                track_wetness, rubber_state, session_time
         FROM live_session_laps";

    match car_idx {
        Some(idx) => {
            let sql = format!("{SELECT} WHERE sub_session_id = ?1 AND car_idx = ?2 ORDER BY car_idx, lap_num");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![sub_session_id, idx], map_row)?
                .collect::<std::result::Result<_, _>>()
                .context("get_laps by car");
            rows
        }
        None => {
            let sql = format!("{SELECT} WHERE sub_session_id = ?1 ORDER BY car_idx, lap_num");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([sub_session_id], map_row)?
                .collect::<std::result::Result<_, _>>()
                .context("get_laps all");
            rows
        }
    }
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
        "SELECT DISTINCT series_id, series_name FROM sessions WHERE series_id IS NOT NULL AND series_name IS NOT NULL ORDER BY series_name",
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
