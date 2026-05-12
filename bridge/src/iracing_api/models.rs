//! Deserialization models for the iRacing Data API responses.

use serde::{Deserialize, Serialize};

// ─── Common ───────────────────────────────────────────────────────────────

/// iRacing Data API endpoints return a `{link}` pre-signed S3 URL.
#[derive(Deserialize)]
pub struct DataApiLink {
    pub link: String,
}

// ─── /data/member/get ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct MemberInfo {
    pub members: Option<Vec<MemberEntry>>,
}

#[derive(Deserialize)]
pub struct MemberEntry {
    pub cust_id: i64,
    pub display_name: String,
}

// ─── /data/results/get ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsessionResult {
    pub subsession_id: i64,
    pub session_id: Option<i64>,
    pub series_id: Option<i64>,
    pub series_name: Option<String>,
    pub season_name: Option<String>,
    pub season_year: Option<i32>,
    pub season_quarter: Option<i32>,
    pub track: Option<TrackInfo>,
    pub event_type: Option<i32>,
    pub event_type_name: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub weather: Option<WeatherInfo>,
    pub session_results: Option<Vec<SimSessionResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub track_id: Option<i64>,
    pub track_name: Option<String>,
    pub config_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherInfo {
    pub weather_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimSessionResult {
    pub simsession_type: Option<i32>,
    pub simsession_name: Option<String>,
    pub results: Option<Vec<DriverEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverEntry {
    pub cust_id: Option<i64>,
    pub display_name: Option<String>,
    pub finish_position: Option<i32>,
    pub starting_position: Option<i32>,
    pub laps_complete: Option<i32>,
    pub incidents: Option<i32>,
    pub best_lap_time: Option<i64>,
    pub average_lap: Option<i64>,
    pub oldi_rating: Option<i32>,
    pub newi_rating: Option<i32>,
    #[serde(rename = "old_sub_level_for_license")]
    pub oldsr: Option<i32>,
    #[serde(rename = "new_sub_level_for_license")]
    pub newsr: Option<i32>,
    pub car_id: Option<i64>,
    pub car_name: Option<String>,
    pub car_class_id: Option<i64>,
    pub car_class_name: Option<String>,
    pub reason_out_id: Option<i32>,
    pub champ_points: Option<i32>,
}

// ─── /data/stats/member_recent_races ─────────────────────────────────────

#[derive(Deserialize)]
pub struct MemberRecentRaces {
    pub races: Option<Vec<RecentRaceEntry>>,
}

#[derive(Deserialize)]
pub struct RecentRaceEntry {
    pub subsession_id: Option<i64>,
}
