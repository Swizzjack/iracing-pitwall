//! YAML-SessionInfo Datenmodell.
//!
//! Nur die Felder, die wir fürs Dashboard brauchen. iRacing emittet deutlich
//! mehr (CameraInfo, RadioInfo, CarSetup); wir ignorieren diese via
//! `#[serde(default)]` und `deny_unknown_fields` NICHT setzen.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Tolerant color deserializer: accepts integer, valid "0x…" hex string, or any
/// garbage string (e.g. "0xundefined" from iRacing offline-test sessions) → None.
fn deserialize_color<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ColorVisitor;
    impl<'de> serde::de::Visitor<'de> for ColorVisitor {
        type Value = Option<i64>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "an integer or hex color string")
        }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Option<i64>, E> { Ok(Some(v)) }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Option<i64>, E> { Ok(Some(v as i64)) }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Option<i64>, E> {
            let hex = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")).unwrap_or(v);
            Ok(i64::from_str_radix(hex, 16).ok())
        }
        fn visit_none<E: serde::de::Error>(self) -> Result<Option<i64>, E> { Ok(None) }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Option<i64>, E> { Ok(None) }
    }
    deserializer.deserialize_any(ColorVisitor)
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct SessionInfoYaml {
    #[serde(rename = "WeekendInfo")]
    pub weekend_info: WeekendInfo,
    #[serde(rename = "SessionInfo")]
    pub session_info: SessionInfoBlock,
    #[serde(rename = "DriverInfo")]
    pub driver_info: DriverInfo,
    #[serde(rename = "SplitTimeInfo", default)]
    pub split_time_info: Option<SplitTimeInfo>,
}

impl SessionInfoYaml {
    /// Returns sorted sector start percentages > 0.0 (S/F at 0.0 is drawn separately).
    pub fn sector_starts(&self) -> Vec<f32> {
        let Some(ref sti) = self.split_time_info else { return vec![] };
        let mut starts: Vec<f32> = sti
            .sectors
            .iter()
            .map(|s| s.sector_start_pct)
            .filter(|&p| p > 0.001)
            .collect();
        starts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        starts.dedup_by(|a, b| (*a - *b).abs() < 0.001);
        starts
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct SplitTimeInfo {
    #[serde(rename = "Sectors", default)]
    pub sectors: Vec<SectorInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "PascalCase")]
pub struct SectorInfo {
    pub sector_num: i32,
    pub sector_start_pct: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct WeekendInfo {
    #[serde(rename = "TrackName")]
    pub track_name: String,
    #[serde(rename = "TrackDisplayName", default)]
    pub track_display_name: String,
    #[serde(rename = "TrackID", default)]
    pub track_id: i64,
    #[serde(rename = "TrackConfigName", default)]
    pub track_config_name: String,
    #[serde(rename = "TrackLength", default)]
    pub track_length: String,
    #[serde(rename = "SeriesID", default)]
    pub series_id: i32,
    #[serde(rename = "SessionID", default)]
    pub session_id: i64,
    #[serde(rename = "SubSessionID", default)]
    pub sub_session_id: i64,
    #[serde(rename = "TrackWeatherType", default)]
    pub track_weather_type: Option<String>,
    #[serde(rename = "TrackCity", default)]
    pub track_city: Option<String>,
    #[serde(rename = "TrackCountry", default)]
    pub track_country: Option<String>,
    #[serde(rename = "TrackAltitude", default)]
    pub track_altitude: Option<String>,
    #[serde(rename = "TrackNumTurns", default)]
    pub track_num_turns: Option<i32>,
    #[serde(rename = "TrackPitSpeedLimit", default)]
    pub track_pit_speed_limit: Option<String>,
    #[serde(rename = "Category", default)]
    pub category: Option<String>,
    #[serde(rename = "WeekendOptions", default)]
    pub weekend_options: Option<WeekendOptions>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct WeekendOptions {
    /// Raw string from iRacing: "unlimited" or e.g. "17x". Use `incident_limit()` to parse.
    #[serde(rename = "IncidentLimit", default)]
    pub incident_limit_raw: String,
}

impl WeekendOptions {
    /// Returns `None` for "unlimited", `Some(n)` for a numeric limit ("17x" → 17).
    pub fn incident_limit(&self) -> Option<u32> {
        if self.incident_limit_raw.eq_ignore_ascii_case("unlimited") || self.incident_limit_raw.is_empty() {
            return None;
        }
        // iRacing appends "x" (e.g. "17x") — strip it and parse the number
        let s = self.incident_limit_raw.trim_end_matches('x').trim();
        s.parse::<u32>().ok()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct SessionInfoBlock {
    #[serde(rename = "Sessions", default)]
    pub sessions: Vec<SessionEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct SessionEntry {
    #[serde(rename = "SessionNum")]
    pub session_num: i32,
    #[serde(rename = "SessionType", default)]
    pub session_type: String,
    #[serde(rename = "SessionName", default)]
    pub session_name: String,
    #[serde(rename = "SessionTrackRubberState", default)]
    pub session_track_rubber_state: Option<String>,
    #[serde(rename = "ResultsPositions", default)]
    pub results_positions: Option<Vec<ResultPosition>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "PascalCase")]
pub struct ResultPosition {
    pub position: i32,
    pub class_position: i32,
    pub car_idx: i32,
    pub lap: i32,
    pub time: f64,
    pub fastest_lap: i32,
    pub fastest_time: f64,
    pub last_time: f64,
    pub laps_led: i32,
    pub laps_complete: i32,
    pub laps_driven: f64,
    pub incidents: i32,
    pub reason_out_id: i32,
    pub reason_out_str: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct DriverInfo {
    #[serde(rename = "DriverCarIdx", default)]
    pub driver_car_idx: i32,
    #[serde(rename = "Drivers", default)]
    pub drivers: Vec<DriverEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "PascalCase")]
pub struct DriverEntry {
    pub car_idx: i32,
    pub user_name: String,
    pub car_number: String,
    #[serde(rename = "CarClassID")]
    pub car_class_id: i32,
    pub car_class_short_name: Option<String>,
    #[serde(rename = "CarClassColor", default, deserialize_with = "deserialize_color")]
    pub car_class_color: Option<i64>,
    #[serde(rename = "CarPath", default)]
    pub car_path: Option<String>,
    #[serde(rename = "CarScreenNameShort", default)]
    pub car_screen_name_short: Option<String>,
    #[serde(rename = "IRating", default)]
    pub irating: i32,
    #[serde(rename = "LicString", default)]
    pub lic_string: String,
    #[serde(rename = "LicColor", default, deserialize_with = "deserialize_color")]
    pub lic_color: Option<i64>,
    #[serde(rename = "CurDriverIncidentCount", default)]
    pub cur_driver_incident_count: i32,
    #[serde(rename = "TeamIncidentCount", default)]
    pub team_incident_count: i32,
    #[serde(rename = "IsSpectator", default)]
    pub is_spectator: i32,
    #[serde(rename = "CarIsPaceCar", default)]
    pub car_is_pace_car: i32,
}
