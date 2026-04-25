//! 60-Hz-Snapshot: kuratierte Whitelist der Telemetrie-Variablen.
//!
//! Whitelist-Design: nur Felder, die das Dashboard konsumiert. Jede
//! neue Anzeige braucht ein Feld hier + in `build()` die Extraction.

use crate::error::Result;
use crate::iracing_sdk::IRacingClient;
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
    pub fn build(client: &IRacingClient) -> Result<Self> {
        Ok(Self {
            // Session state
            session_num: client.get_i32("SessionNum")?,
            session_state: client.get_i32("SessionState")?,
            session_flags: client.get_bitfield("SessionFlags")?,
            is_on_track: client.get_bool("IsOnTrack")?,
            is_on_track_car: client.get_bool("IsOnTrackCar")?,
            is_in_garage: client.get_bool("IsInGarage")?,
            player_car_idx: client.get_i32("PlayerCarIdx")?,

            // Inputs
            throttle: client.get_f32("Throttle")?,
            brake: client.get_f32("Brake")?,
            clutch: client.get_f32("Clutch")?,
            steering_wheel_angle: client.get_f32("SteeringWheelAngle")?,
            gear: client.get_i32("Gear")?,
            rpm: client.get_f32("RPM")?,
            speed_ms: client.get_f32("Speed")?,

            // Fuel
            fuel_level: client.get_f32("FuelLevel")?,
            fuel_level_pct: client.get_f32("FuelLevelPct")?,
            fuel_use_per_hour: client.get_f32("FuelUsePerHour")?,

            // Lap
            lap: client.get_i32("Lap")?,
            lap_dist_pct: client.get_f32("LapDistPct")?,
            lap_current_time: client.get_f32("LapCurrentLapTime")?,
            lap_last_time: client.get_f32("LapLastLapTime")?,
            lap_best_time: client.get_f32("LapBestLapTime")?,
            lap_delta_to_best: client.get_f32("LapDeltaToBestLap")?,

            // Position
            player_position: client.get_i32("PlayerCarPosition")?,
            player_class_position: client.get_i32("PlayerCarClassPosition")?,
            on_pit_road: client.get_bool("OnPitRoad")?,

            // Tires
            tire_temp_lf: corner_temps(client, "LF")?,
            tire_temp_rf: corner_temps(client, "RF")?,
            tire_temp_lr: corner_temps(client, "LR")?,
            tire_temp_rr: corner_temps(client, "RR")?,
            tire_pressure: [
                client.get_f32("LFcoldPressure")?,
                client.get_f32("RFcoldPressure")?,
                client.get_f32("LRcoldPressure")?,
                client.get_f32("RRcoldPressure")?,
            ],
            tire_wear_lf: corner_wear(client, "LF")?,
            tire_wear_rf: corner_wear(client, "RF")?,
            tire_wear_lr: corner_wear(client, "LR")?,
            tire_wear_rr: corner_wear(client, "RR")?,

            // Engine
            water_temp: client.get_f32("WaterTemp")?,
            oil_temp: client.get_f32("OilTemp")?,
            oil_press: client.get_f32("OilPress")?,
            voltage: client.get_f32("Voltage")?,
            engine_warnings: client.get_bitfield("EngineWarnings")?,
            brake_bias: client.get_f32("dcBrakeBias")?,

            // G-Forces
            lat_accel: client.get_f32("LatAccel")?,
            long_accel: client.get_f32("LongAccel")?,
            vert_accel: client.get_f32("VertAccel")?,
            yaw: client.get_f32("Yaw")?,
            pitch: client.get_f32("Pitch")?,
            roll: client.get_f32("Roll")?,

            // Track/Weather
            track_temp: client.get_f32("TrackTempCrew")?,
            air_temp: client.get_f32("AirTemp")?,
        })
    }
}

fn corner_temps(client: &IRacingClient, prefix: &str) -> Result<[f32; 3]> {
    Ok([
        client.get_f32(&format!("{prefix}tempL"))?,
        client.get_f32(&format!("{prefix}tempM"))?,
        client.get_f32(&format!("{prefix}tempR"))?,
    ])
}

fn corner_wear(client: &IRacingClient, prefix: &str) -> Result<[f32; 3]> {
    Ok([
        client.get_f32(&format!("{prefix}wearL"))?,
        client.get_f32(&format!("{prefix}wearM"))?,
        client.get_f32(&format!("{prefix}wearR"))?,
    ])
}
