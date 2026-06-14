use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;

use crate::error::Result;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::iracing_sdk::IRacingClient;
use crate::telemetry::track_recorder::{TrackRecorder, TrackShape};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct TrackMapSnapshot {
    pub track_key: String,
    /// None while recording is in progress.
    pub shape: Option<TrackShape>,
    pub cars: Vec<TrackCar>,
    pub player_car_idx: i32,
    /// Sorted sector start percentages from YAML SplitTimeInfo (excludes 0.0 = S/F line).
    pub sectors: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct TrackCar {
    pub car_idx: i32,
    pub lap_dist_pct: f32,
    pub class_id: i32,
    pub class_position: i32,
    /// 24-bit RGB from iRacing YAML (e.g. 0xFF0000 = red). None if not set.
    pub class_color: Option<i64>,
    pub on_pit_road: bool,
    /// CarIdxTrackSurface: -1=NotInWorld, 0=OffTrack, 1=InPitStall, 2=ApproachingPits, 3=OnTrack
    pub surface: i32,
}

impl TrackMapSnapshot {
    pub fn build(
        client: &IRacingClient,
        yaml: &SessionInfoYaml,
        recorder: &TrackRecorder,
        player_car_idx: i32,
    ) -> Result<Self> {
        let track_key = crate::telemetry::track_recorder::track_key(&yaml.weekend_info);

        let lap_dist_pcts = client.get_f32_array("CarIdxLapDistPct")?;
        let class_positions = client.get_i32_array("CarIdxClassPosition")?;
        let on_pit = client.get_bool_array("CarIdxOnPitRoad")?;
        let surfaces = client.get_i32_array("CarIdxTrackSurface").ok();

        // Build carIdx → (class_id, class_color) lookup from YAML.
        let class_info: HashMap<i32, (i32, Option<i64>)> = yaml
            .driver_info
            .drivers
            .iter()
            .map(|d| (d.car_idx, (d.car_class_id, d.car_class_color)))
            .collect();

        let cars: Vec<TrackCar> = yaml
            .driver_info
            .drivers
            .iter()
            .filter_map(|driver| {
                let idx = driver.car_idx as usize;
                let surface = surfaces
                    .as_ref()
                    .and_then(|arr| arr.get(idx).copied())
                    .unwrap_or(0);
                // Skip cars not in world.
                if surface == -1 {
                    return None;
                }
                let lap_dist_pct = *lap_dist_pcts.get(idx).unwrap_or(&0.0);
                let class_pos = *class_positions.get(idx).unwrap_or(&0);
                // Skip cars that haven't entered (class_pos == 0 and not on track).
                if class_pos == 0 && surface != 3 {
                    return None;
                }
                let &(class_id, class_color) = class_info.get(&driver.car_idx)?;
                Some(TrackCar {
                    car_idx: driver.car_idx,
                    lap_dist_pct,
                    class_id,
                    class_position: class_pos,
                    class_color,
                    on_pit_road: *on_pit.get(idx).unwrap_or(&false),
                    surface,
                })
            })
            .collect();

        Ok(TrackMapSnapshot {
            track_key,
            shape: recorder.shape.clone(),
            cars,
            player_car_idx,
            sectors: yaml.sector_starts(),
        })
    }
}
