//! YAML-SessionInfo Datenmodell.
//!
//! Nur die Felder, die wir fürs Dashboard brauchen. iRacing emittet deutlich
//! mehr (CameraInfo, RadioInfo, SplitTimeInfo, CarSetup); wir ignorieren diese
//! via `#[serde(default)]` und `deny_unknown_fields` NICHT setzen.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct SessionInfoYaml {
    #[serde(rename = "WeekendInfo")]
    pub weekend_info: WeekendInfo,
    #[serde(rename = "SessionInfo")]
    pub session_info: SessionInfoBlock,
    #[serde(rename = "DriverInfo")]
    pub driver_info: DriverInfo,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
pub struct WeekendInfo {
    #[serde(rename = "TrackName")]
    pub track_name: String,
    #[serde(rename = "TrackDisplayName", default)]
    pub track_display_name: String,
    #[serde(rename = "SeriesID", default)]
    pub series_id: i32,
    #[serde(rename = "SessionID", default)]
    pub session_id: i64,
    #[serde(rename = "SubSessionID", default)]
    pub sub_session_id: i64,
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
    #[serde(rename = "IRating", default)]
    pub irating: i32,
}
