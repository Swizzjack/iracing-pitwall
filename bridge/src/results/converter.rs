//! Converts iRacing API models to persistence row types.

use anyhow::Result;

use crate::iracing_api::models::SubsessionResult;
use crate::persistence::queries::{ResultRow, SegmentRow, SessionRow};
use super::unix_now;

/// iRacing simsession_type values.
const SIM_TYPE_PRACTICE: i32 = 0;
const SIM_TYPE_QUALIFY: i32 = 4;
const SIM_TYPE_RACE: i32 = 6;

fn simsession_label(t: i32) -> &'static str {
    match t {
        SIM_TYPE_PRACTICE => "P",
        SIM_TYPE_QUALIFY => "Q",
        SIM_TYPE_RACE => "R",
        _ => "?",
    }
}

fn parse_iracing_time(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

pub fn convert(
    sub_session_id: i64,
    result: SubsessionResult,
    raw_json: String,
) -> Result<(SessionRow, Vec<SegmentRow>, Vec<ResultRow>)> {
    let track_id = result.track.as_ref().and_then(|t| t.track_id);
    let track_name = result.track.as_ref().and_then(|t| t.track_name.clone());
    let track_config = result.track.as_ref().and_then(|t| t.config_name.clone());
    let weather_summary = result
        .weather
        .as_ref()
        .and_then(|w| w.weather_summary.clone());

    let start_time = result
        .start_time
        .as_deref()
        .and_then(parse_iracing_time)
        .unwrap_or_else(unix_now);
    let end_time = result.end_time.as_deref().and_then(parse_iracing_time);
    let event_type_name = result.event_type_name.clone();

    let session_row = SessionRow {
        sub_session_id,
        session_id: result.session_id,
        series_id: result.series_id,
        series_name: result.series_name,
        season_name: result.season_name,
        season_year: result.season_year,
        season_quarter: result.season_quarter,
        track_id,
        track_name,
        track_config,
        event_type: result.event_type,
        event_type_name: None,
        start_time,
        end_time: None,
        weather_summary: None,
        sof: None,
        cars_json: None,
        raw_json: String::new(),
        source: "api".to_string(),
        captured_at: None,
    };

    let mut segment_rows: Vec<SegmentRow> = Vec::new();
    let mut result_rows: Vec<ResultRow> = Vec::new();

    if let Some(sessions) = result.session_results {
        for sim_session in &sessions {
            let label = sim_session
                .simsession_type
                .map(simsession_label)
                .unwrap_or("?");

            // One segment per simsession_type in the API result.
            if !segment_rows.iter().any(|s: &SegmentRow| s.simsession_type == label) {
                segment_rows.push(SegmentRow {
                    sub_session_id,
                    simsession_type: label.to_string(),
                    simsession_num: 0,
                    event_type_name: event_type_name.clone(),
                    start_time,
                    end_time,
                    weather_summary: weather_summary.clone(),
                    sof: None,
                    raw_json: Some(raw_json.clone()),
                    source: "api".to_string(),
                    captured_at: None,
                    is_finalized: true,
                });
            }

            if let Some(entries) = &sim_session.results {
                for entry in entries {
                    let cust_id = match entry.cust_id {
                        Some(id) => id,
                        None => continue,
                    };
                    result_rows.push(ResultRow {
                        sub_session_id,
                        simsession_type: label.to_string(),
                        // API-sourced rows have no car_idx; use -1 as sentinel.
                        car_idx: -1,
                        cust_id: Some(cust_id),
                        display_name: entry.display_name.clone(),
                        finish_position: entry.finish_position,
                        starting_position: entry.starting_position,
                        class_position: None,
                        laps_complete: entry.laps_complete,
                        incidents: entry.incidents,
                        best_lap_ms: entry.best_lap_time.map(|v| v as i32),
                        average_lap_ms: entry.average_lap.map(|v| v as i32),
                        oldi_rating: entry.oldi_rating,
                        newi_rating: entry.newi_rating,
                        oldsr: entry.oldsr,
                        newsr: entry.newsr,
                        car_id: entry.car_id,
                        car_name: entry.car_name.clone(),
                        car_class_id: entry.car_class_id,
                        car_class_name: entry.car_class_name.clone(),
                        is_player: false,
                        last_sectors_json: None,
                        best_sectors_json: None,
                        pit_stops: None,
                        tire_compound: None,
                        car_number: None,
                        safety_rating: None,
                        lic_color: None,
                        car_class_color: None,
                        reason_out: None,
                        source: "api".to_string(),
                    });
                }
            }
        }
    }

    // Fallback: if no session_results, create a single 'R' segment.
    if segment_rows.is_empty() {
        segment_rows.push(SegmentRow {
            sub_session_id,
            simsession_type: "R".to_string(),
            simsession_num: 0,
            event_type_name,
            start_time,
            end_time,
            weather_summary,
            sof: None,
            raw_json: Some(raw_json),
            source: "api".to_string(),
            captured_at: None,
            is_finalized: true,
        });
    }

    Ok((session_row, segment_rows, result_rows))
}

/// Given a list of result rows and the local player's `cust_id`, marks `is_player`.
pub fn mark_player(rows: &mut Vec<ResultRow>, cust_id: i64) {
    for row in rows.iter_mut() {
        row.is_player = row.cust_id == Some(cust_id);
    }
}
