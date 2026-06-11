//! 60 Hz snapshot: curated whitelist of telemetry variables.
//!
//! Whitelist design: only fields the dashboard consumes. Every new
//! display needs a field here plus the extraction in `build()`.

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
    pub player_car_my_incident_count: Option<i32>,

    // Per-car session flags for the player (blue flag, drive-through penalty, etc.)
    pub player_car_session_flags: Option<u32>,
    /// True when iRacing has issued a drive-through penalty for the player's car.
    /// Bit 0x10000000 of CarIdxSessionFlags — note this is NOT part of the
    /// public irsdk_defines.h enum (which lists 0x10000000 as irsdk_startHidden);
    /// determined empirically via the SDK debug overlay.
    pub player_has_drivethrough_penalty: bool,

    // Tires — 4 corners × (carcass temp L/M/R, pressure, wear L/M/R)
    pub tire_temp_lf: [f32; 3],
    pub tire_temp_rf: [f32; 3],
    pub tire_temp_lr: [f32; 3],
    pub tire_temp_rr: [f32; 3],
    pub tire_cold_pressure: [f32; 4], // LF, RF, LR, RR — setup cold target
    pub tire_wear_lf: [f32; 3],
    pub tire_wear_rf: [f32; 3],
    pub tire_wear_lr: [f32; 3],
    pub tire_wear_rr: [f32; 3],
    // Extended tire vars (car-class specific — None if car doesn't expose them)
    pub tire_pressure: Option<[f32; 4]>,         // live hot pressure LF/RF/LR/RR
    pub tire_temp_surface_lf: Option<[f32; 3]>,  // surface temps inner/mid/outer
    pub tire_temp_surface_rf: Option<[f32; 3]>,
    pub tire_temp_surface_lr: Option<[f32; 3]>,
    pub tire_temp_surface_rr: Option<[f32; 3]>,
    pub tire_speed: Option<[f32; 4]>,            // wheel rotation rad/s LF/RF/LR/RR
    pub tire_ride_height: Option<[f32; 4]>,      // ride height LF/RF/LR/RR

    // Engine
    pub water_temp: f32,
    pub oil_temp: f32,
    pub oil_press: f32,
    pub voltage: f32,
    pub engine_warnings: u32,
    pub brake_bias: Option<f32>,

    // G-Forces
    pub lat_accel: f32,
    pub long_accel: f32,
    pub vert_accel: f32,
    pub yaw: f32,
    pub yaw_north: Option<f32>,
    pub pitch: f32,
    pub roll: f32,

    // Track/Weather
    pub track_temp: f32,
    pub air_temp: f32,
    pub skies: Option<i32>,
    pub track_wetness: Option<i32>,
    pub weather_declared_wet: Option<bool>,
    pub precipitation: Option<f32>,
    pub air_pressure: Option<f32>,
    pub air_density: Option<f32>,
    pub relative_humidity: Option<f32>,
    pub fog_level: Option<f32>,
    pub wind_vel: Option<f32>,
    pub wind_dir: Option<f32>,
    pub session_time_of_day: Option<f32>,

    // Race format / remaining (32767 = unlimited / time-based)
    pub session_time_remain: Option<f64>,
    pub session_laps_remain: Option<i32>,
    pub session_time_total: Option<f64>,
    pub session_laps_total: Option<i32>,

    // Battery / Hybrid (all Option — car-specific, not universally available)
    pub energy_battery_pct: Option<f32>,
    pub energy_battery: Option<f32>,
    pub mguk_deploy_mode: Option<i32>,
    pub power_mguk: Option<f32>,

    // Driver Adjustments (Option — car-class specific)
    pub dc_traction_control: Option<f32>,
    pub dc_traction_control_2: Option<f32>,
    pub dc_abs: Option<f32>,
    pub dc_throttle_shape: Option<f32>,
    pub dc_diff_entry: Option<f32>,
    pub dc_diff_middle: Option<f32>,
    pub dc_diff_exit: Option<f32>,
    pub dc_anti_roll_front: Option<f32>,
    pub dc_anti_roll_rear: Option<f32>,
    pub dc_engine_braking: Option<f32>,

    // DRS / Push-to-pass
    pub drs_status: Option<i32>,
    pub p2p_count: Option<i32>,
    pub p2p_status: Option<bool>,

    // Engine extras (Option — turbo/fuel-injection cars only)
    pub manifold_press: Option<f32>,
    pub fuel_press: Option<f32>,

    // Shift light thresholds (set per car, exposed as telemetry vars)
    pub shift_light_first_rpm: Option<f32>,
    pub shift_light_shift_rpm: Option<f32>,
    pub shift_light_last_rpm: Option<f32>,
    pub shift_light_blink_rpm: Option<f32>,
}

impl TelemetrySnapshot {
    /// Extracts the whitelisted fields from the current frame buffer.
    /// Must be called after `IRacingClient::wait_for_frame()`.
    pub fn build(client: &IRacingClient) -> Result<Self> {
        let player_car_idx = client.get_i32("PlayerCarIdx")?;

        // Per-car session flags for the player's car (BitField array — blue/debris flag detection)
        let player_car_session_flags = client
            .get_bitfield_array("CarIdxSessionFlags")
            .ok()
            .and_then(|arr| arr.get(player_car_idx as usize).copied());

        Ok(Self {
            // Session state
            session_num: client.get_i32("SessionNum")?,
            session_state: client.get_i32("SessionState")?,
            session_flags: client.get_bitfield("SessionFlags")?,
            is_on_track: client.get_bool("IsOnTrack")?,
            is_on_track_car: client.get_bool("IsOnTrackCar")?,
            is_in_garage: client.get_bool("IsInGarage")?,
            player_car_idx,

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
            player_car_my_incident_count: client.get_i32("PlayerCarMyIncidentCount").ok(),
            player_car_session_flags,
            player_has_drivethrough_penalty: player_car_session_flags
                .is_some_and(|f| (f & 0x10000000) != 0),

            // Tires — var names spelled out as literals: this runs at 60 Hz,
            // so no per-frame format!() allocations, and the names stay greppable.
            tire_temp_lf: read3(client, ["LFtempCL", "LFtempCM", "LFtempCR"])?,
            tire_temp_rf: read3(client, ["RFtempCL", "RFtempCM", "RFtempCR"])?,
            tire_temp_lr: read3(client, ["LRtempCL", "LRtempCM", "LRtempCR"])?,
            tire_temp_rr: read3(client, ["RRtempCL", "RRtempCM", "RRtempCR"])?,
            tire_cold_pressure: [
                client.get_f32("LFcoldPressure")?,
                client.get_f32("RFcoldPressure")?,
                client.get_f32("LRcoldPressure")?,
                client.get_f32("RRcoldPressure")?,
            ],
            tire_wear_lf: read3(client, ["LFwearL", "LFwearM", "LFwearR"])?,
            tire_wear_rf: read3(client, ["RFwearL", "RFwearM", "RFwearR"])?,
            tire_wear_lr: read3(client, ["LRwearL", "LRwearM", "LRwearR"])?,
            tire_wear_rr: read3(client, ["RRwearL", "RRwearM", "RRwearR"])?,
            tire_pressure: read4_opt(client, ["LFpressure", "RFpressure", "LRpressure", "RRpressure"]),
            tire_temp_surface_lf: read3_opt(client, ["LFtempL", "LFtempM", "LFtempR"]),
            tire_temp_surface_rf: read3_opt(client, ["RFtempL", "RFtempM", "RFtempR"]),
            tire_temp_surface_lr: read3_opt(client, ["LRtempL", "LRtempM", "LRtempR"]),
            tire_temp_surface_rr: read3_opt(client, ["RRtempL", "RRtempM", "RRtempR"]),
            tire_speed: read4_opt(client, ["LFspeed", "RFspeed", "LRspeed", "RRspeed"]),
            tire_ride_height: read4_opt(client, ["LFrideHeight", "RFrideHeight", "LRrideHeight", "RRrideHeight"]),

            // Engine
            water_temp: client.get_f32("WaterTemp")?,
            oil_temp: client.get_f32("OilTemp")?,
            oil_press: client.get_f32("OilPress")?,
            voltage: client.get_f32("Voltage")?,
            engine_warnings: client.get_bitfield("EngineWarnings")?,
            brake_bias: client.get_f32("dcBrakeBias").ok(),

            // G-Forces
            lat_accel: client.get_f32("LatAccel")?,
            long_accel: client.get_f32("LongAccel")?,
            vert_accel: client.get_f32("VertAccel")?,
            yaw: client.get_f32("Yaw")?,
            yaw_north: client.get_f32("YawNorth").ok(),
            pitch: client.get_f32("Pitch")?,
            roll: client.get_f32("Roll")?,

            // Track/Weather
            track_temp: client.get_f32("TrackTempCrew")?,
            air_temp: client.get_f32("AirTemp")?,
            skies: client.get_i32("Skies").ok(),
            track_wetness: client.get_i32("TrackWetness").ok(),
            weather_declared_wet: client.get_bool("WeatherDeclaredWet").ok(),
            precipitation: client.get_f32("Precipitation").ok(),
            air_pressure: client.get_f32("AirPressure").ok(),
            air_density: client.get_f32("AirDensity").ok(),
            relative_humidity: client.get_f32("RelativeHumidity").ok(),
            fog_level: client.get_f32("FogLevel").ok(),
            wind_vel: client.get_f32("WindVel").ok(),
            wind_dir: client.get_f32("WindDir").ok(),
            session_time_of_day: client.get_f32("SessionTimeOfDay").ok(),

            // Race format / remaining
            session_time_remain: client.get_f64("SessionTimeRemain").ok(),
            session_laps_remain: client.get_i32("SessionLapsRemain").ok(),
            session_time_total: client.get_f64("SessionTimeTotal").ok(),
            session_laps_total: client.get_i32("SessionLapsTotal").ok(),

            // Battery / Hybrid
            energy_battery_pct: client.get_f32("EnergyERSBatteryPct").ok(),
            energy_battery: client.get_f32("EnergyERSBattery").ok(),
            mguk_deploy_mode: client.get_i32("dcMGUKDeployMode").ok(),
            power_mguk: client.get_f32("PowerMGUK").ok(),

            // Driver Adjustments
            dc_traction_control: client.get_f32("dcTractionControl").ok(),
            dc_traction_control_2: client.get_f32("dcTractionControl2").ok(),
            dc_abs: client.get_f32("dcABS").ok(),
            dc_throttle_shape: client.get_f32("dcThrottleShape").ok(),
            dc_diff_entry: client.get_f32("dcDiffEntry").ok(),
            dc_diff_middle: client.get_f32("dcDiffMiddle").ok(),
            dc_diff_exit: client.get_f32("dcDiffExit").ok(),
            dc_anti_roll_front: client.get_f32("dcAntiRollFront").ok(),
            dc_anti_roll_rear: client.get_f32("dcAntiRollRear").ok(),
            dc_engine_braking: client.get_f32("dcEngineBraking").ok(),

            // DRS / Push-to-pass
            drs_status: client.get_i32("DrsStatus").ok(),
            p2p_count: client.get_i32("P2P_Count").ok(),
            p2p_status: client.get_bool("P2P_Status").ok(),

            // Engine extras
            manifold_press: client.get_f32("ManifoldPress").ok(),
            fuel_press: client.get_f32("FuelPress").ok(),

            // Shift light thresholds
            shift_light_first_rpm: client.get_f32("PlayerCarSLFirstRPM").ok(),
            shift_light_shift_rpm: client.get_f32("PlayerCarSLShiftRPM").ok(),
            shift_light_last_rpm: client.get_f32("PlayerCarSLLastRPM").ok(),
            shift_light_blink_rpm: client.get_f32("PlayerCarSLBlinkRPM").ok(),
        })
    }
}

fn read3(client: &IRacingClient, names: [&str; 3]) -> Result<[f32; 3]> {
    Ok([
        client.get_f32(names[0])?,
        client.get_f32(names[1])?,
        client.get_f32(names[2])?,
    ])
}

/// `None` if any of the vars is missing (car class doesn't expose them).
fn read3_opt(client: &IRacingClient, names: [&str; 3]) -> Option<[f32; 3]> {
    Some([
        client.get_f32(names[0]).ok()?,
        client.get_f32(names[1]).ok()?,
        client.get_f32(names[2]).ok()?,
    ])
}

/// `None` if any of the vars is missing (car class doesn't expose them).
fn read4_opt(client: &IRacingClient, names: [&str; 4]) -> Option<[f32; 4]> {
    Some([
        client.get_f32(names[0]).ok()?,
        client.get_f32(names[1]).ok()?,
        client.get_f32(names[2]).ok()?,
        client.get_f32(names[3]).ok()?,
    ])
}
