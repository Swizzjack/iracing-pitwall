//! 60-Hz-Snapshot: kuratierte Whitelist der Telemetrie-Variablen.
//!
//! Whitelist-Design: nur Felder, die das Dashboard konsumiert. Jede
//! neue Anzeige braucht ein Feld hier + in `build()` die Extraction.

use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySnapshot {
    // Session state
    pub session_num: i32,
    pub session_state: i32,
    pub session_flags: u32,
    pub is_on_track: bool,
    pub is_on_track_car: bool,
    pub is_in_garage: bool,
    pub player_car_idx: i32,

    // Inputs
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub steering_wheel_angle: f32,
    pub gear: i32,
    pub rpm: f32,
    pub speed_ms: f32,

    // Fuel
    pub fuel_level: f32,
    pub fuel_level_pct: f32,
    pub fuel_use_per_hour: f32,

    // Lap
    pub lap: i32,
    pub lap_dist_pct: f32,
    pub lap_current_time: f32,
    pub lap_last_time: f32,
    pub lap_best_time: f32,
    pub lap_delta_to_best: f32,

    // Position
    pub player_position: i32,
    pub player_class_position: i32,
    pub on_pit_road: bool,

    // Tires — 4 corners × (temp L/M/R, pressure, wear L/M/R)
    pub tire_temp_lf: [f32; 3],
    pub tire_temp_rf: [f32; 3],
    pub tire_temp_lr: [f32; 3],
    pub tire_temp_rr: [f32; 3],
    pub tire_pressure: [f32; 4], // LF, RF, LR, RR
    pub tire_wear_lf: [f32; 3],
    pub tire_wear_rf: [f32; 3],
    pub tire_wear_lr: [f32; 3],
    pub tire_wear_rr: [f32; 3],

    // Engine
    pub water_temp: f32,
    pub oil_temp: f32,
    pub oil_press: f32,
    pub voltage: f32,
    pub engine_warnings: u32,
    pub brake_bias: f32,

    // G-Forces
    pub lat_accel: f32,
    pub long_accel: f32,
    pub vert_accel: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,

    // Track/Weather
    pub track_temp: f32,
    pub air_temp: f32,
}

impl TelemetrySnapshot {
    /// Extrahiert die Whitelist-Felder aus dem aktuellen Frame-Buffer.
    /// Muss nach `IRacingClient::wait_for_frame()` aufgerufen werden.
    pub fn build() -> Self {
        todo!("read each whitelisted var via IRacingClient::get_*() and populate")
    }
}
