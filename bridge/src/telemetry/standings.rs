//! 4-Hz-Snapshot: CarIdx-basierte Standings inkl. berechneter Gaps.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct StandingsSnapshot {
    pub session_num: i32,
    pub session_type: String,
    pub entries: Vec<StandingEntry>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct StandingEntry {
    pub car_idx: i32,
    pub position: i32,
    pub class_position: i32,
    pub user_name: String,
    pub car_number: String,
    pub lap: i32,
    pub lap_dist_pct: f32,
    pub last_lap_time: f32,
    pub best_lap_time: f32,
    /// Sekunden hinter Leader. `None` = noch keine valide Rundenzeit.
    pub gap_to_leader: Option<f32>,
    pub on_pit_road: bool,
    pub incidents: i32,
}

impl StandingsSnapshot {
    pub fn build() -> Self {
        todo!("merge CarIdx* arrays with active-session ResultsPositions + DriverInfo")
    }
}
