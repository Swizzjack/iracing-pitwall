//! 4-Hz-Snapshot: CarIdx-basierte Standings inkl. berechneter Gaps.

use crate::error::Result;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::iracing_sdk::IRacingClient;
use crate::telemetry::pit_tracker::PitTracker;
use serde::Serialize;
use std::collections::HashMap;
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
    pub car_class_id: i32,
    pub car_class_short_name: String,
    pub user_name: String,
    pub car_number: String,
    pub lap: i32,
    pub lap_dist_pct: f32,
    pub last_lap_time: f32,
    pub best_lap_time: f32,
    /// Seconds behind the in-class leader. Race: `CarIdxF2Time`. Practice/Qualify:
    /// `driver.fastest_time − class_leader.fastest_time`. `None` = no valid data yet.
    pub gap_to_leader: Option<f32>,
    pub on_pit_road: bool,
    pub incidents: i32,
    pub pit_stops: u32,
    pub last_pit_road_sec: Option<f32>,
    pub current_pit_road_sec: Option<f32>,
}

impl StandingsSnapshot {
    /// Builds a standings snapshot by merging live CarIdx telemetry arrays with
    /// YAML DriverInfo (names/numbers) and ResultsPositions (gap calculation).
    pub fn build(
        client: &IRacingClient,
        yaml: &SessionInfoYaml,
        pit_tracker: &PitTracker,
    ) -> Result<Self> {
        let session_num = client.get_i32("SessionNum")?;

        let current_session = yaml
            .session_info
            .sessions
            .iter()
            .find(|s| s.session_num == session_num);

        let session_type = current_session
            .map(|s| s.session_type.clone())
            .unwrap_or_default();

        // CarIdx telemetry arrays — one element per car slot (up to 64).
        let positions = client.get_i32_array("CarIdxPosition")?;
        let class_positions = client.get_i32_array("CarIdxClassPosition")?;
        let laps = client.get_i32_array("CarIdxLap")?;
        let lap_dist_pcts = client.get_f32_array("CarIdxLapDistPct")?;
        let last_lap_times = client.get_f32_array("CarIdxLastLapTime")?;
        let best_lap_times = client.get_f32_array("CarIdxBestLapTime")?;
        let on_pit = client.get_bool_array("CarIdxOnPitRoad")?;
        // Incidents may be absent in older iRacing builds — default to 0.
        let incidents = client.get_i32_array("CarIdxTeamIncidentCount").ok();
        // F2Time = seconds behind in-class leader during a race; absent in older
        // builds and meaningless outside race sessions, so it's optional.
        let f2_times = client.get_f32_array("CarIdxF2Time").ok();

        // Build a car_idx → ResultPosition lookup for fastest-time fallback.
        let results_map: HashMap<i32, &crate::iracing_sdk::types::ResultPosition> = current_session
            .and_then(|s| s.results_positions.as_ref())
            .map(|rp| rp.iter().map(|r| (r.car_idx, r)).collect())
            .unwrap_or_default();

        // Per-class fastest time, used for non-race gap calculation.
        // class_id → min(fastest_time) across drivers of that class with a valid lap.
        let mut class_leader_fastest: HashMap<i32, f64> = HashMap::new();
        for driver in &yaml.driver_info.drivers {
            if let Some(res) = results_map.get(&driver.car_idx) {
                if res.fastest_time > 0.0 {
                    class_leader_fastest
                        .entry(driver.car_class_id)
                        .and_modify(|t| {
                            if res.fastest_time < *t {
                                *t = res.fastest_time;
                            }
                        })
                        .or_insert(res.fastest_time);
                }
            }
        }

        // Race sessions get gaps from CarIdxF2Time (already per-class). Other
        // session types fall back to per-class best-lap delta.
        let is_race = session_type.eq_ignore_ascii_case("Race");

        let mut entries: Vec<StandingEntry> = yaml
            .driver_info
            .drivers
            .iter()
            .filter_map(|driver| {
                let idx = driver.car_idx as usize;
                let pos = *positions.get(idx)?;
                // position == 0 means the car hasn't entered the session yet.
                if pos == 0 {
                    return None;
                }

                let res = results_map.get(&driver.car_idx);
                let class_pos = *class_positions.get(idx).unwrap_or(&0);

                let gap_to_leader: Option<f32> = if is_race {
                    let raw = f2_times.as_ref().and_then(|arr| arr.get(idx).copied());
                    match raw {
                        Some(t) if t > 0.0 => Some(t),
                        Some(t) if t == 0.0 && class_pos == 1 => Some(0.0),
                        _ => None,
                    }
                } else {
                    let driver_fastest = res.map(|r| r.fastest_time).unwrap_or(0.0);
                    let leader_fastest = class_leader_fastest
                        .get(&driver.car_class_id)
                        .copied()
                        .unwrap_or(0.0);
                    if driver_fastest > 0.0 && leader_fastest > 0.0 {
                        Some((driver_fastest - leader_fastest) as f32)
                    } else {
                        None
                    }
                };

                // For drivers who left the server the live CarIdx arrays return -1.
                // Fall back to the session-historical times stored in ResultsPositions.
                let live_last = *last_lap_times.get(idx).unwrap_or(&-1.0);
                let last_lap_time = if live_last > 0.0 {
                    live_last
                } else {
                    res.filter(|r| r.last_time > 0.0)
                        .map(|r| r.last_time as f32)
                        .unwrap_or(-1.0)
                };

                let live_best = *best_lap_times.get(idx).unwrap_or(&-1.0);
                let best_lap_time = if live_best > 0.0 {
                    live_best
                } else {
                    res.filter(|r| r.fastest_time > 0.0)
                        .map(|r| r.fastest_time as f32)
                        .unwrap_or(-1.0)
                };

                let pit = pit_tracker.get(driver.car_idx);
                Some(StandingEntry {
                    car_idx: driver.car_idx,
                    position: pos,
                    class_position: class_pos,
                    car_class_id: driver.car_class_id,
                    car_class_short_name: driver.car_class_short_name.clone().unwrap_or_default(),
                    user_name: driver.user_name.clone(),
                    car_number: driver.car_number.clone(),
                    lap: *laps.get(idx).unwrap_or(&0),
                    lap_dist_pct: *lap_dist_pcts.get(idx).unwrap_or(&0.0),
                    last_lap_time,
                    best_lap_time,
                    gap_to_leader,
                    on_pit_road: *on_pit.get(idx).unwrap_or(&false),
                    incidents: incidents.and_then(|arr| arr.get(idx).copied()).unwrap_or(0),
                    pit_stops: pit.map_or(0, |p| p.pit_stops),
                    last_pit_road_sec: pit.and_then(|p| p.last_pit_road_sec),
                    current_pit_road_sec: pit.and_then(|p| p.current_pit_road_sec),
                })
            })
            .collect();

        entries.sort_unstable_by_key(|e| e.position);

        Ok(Self {
            session_num,
            session_type,
            entries,
        })
    }
}
