//! EngineerState — a 10 Hz snapshot derived from iRacing telemetry.
//!
//! `StateAggregator::build_state()` converts `TelemetrySnapshot` +
//! `StandingsSnapshot` into the flat struct the rule engine operates on.

use std::collections::VecDeque;
use std::time::Duration;

use crate::telemetry::{StandingsSnapshot, TelemetrySnapshot};

// ---------------------------------------------------------------------------
// Session enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Practice,
    Qualifying,
    Race,
    Unknown,
}

impl SessionType {
    pub fn from_str(s: &str) -> Self {
        let l = s.to_ascii_lowercase();
        if l.contains("race") {
            SessionType::Race
        } else if l.contains("qualify") || l.contains("lone qual") {
            SessionType::Qualifying
        } else if l.contains("practice") || l.contains("open prac") {
            SessionType::Practice
        } else {
            SessionType::Unknown
        }
    }

    pub fn to_mask(self) -> super::rules::SessionMask {
        use super::rules::SessionMask;
        match self {
            SessionType::Practice => SessionMask::PRACTICE,
            SessionType::Qualifying => SessionMask::QUALIFYING,
            SessionType::Race => SessionMask::RACE,
            SessionType::Unknown => SessionMask::empty(),
        }
    }
}

/// Session phase derived from iRacing `SessionState` telemetry var.
///
/// iRacing SessionState values:
///   0 = Invalid, 1 = GetInCar, 2 = Warmup, 3 = ParadeLaps,
///   4 = Racing, 5 = Checkered, 6 = CoolDown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Unknown,
    GettingInCar, // GetInCar (1) — driver entering the vehicle
    Formation,    // ParadeLaps (3)
    Racing,       // Racing (4)
    Checkered,    // Checkered (5) — race may still be going
    Finished,     // CoolDown (6)
    RedFlag,      // Any state + red flag bit in session_flags
}

impl SessionPhase {
    pub fn from_iracing(session_state: i32, session_flags: u32) -> Self {
        // iRacing SessionFlags: bit 0x10 = Red flag
        const RED_FLAG: u32 = 0x0010;
        if session_flags & RED_FLAG != 0 {
            return SessionPhase::RedFlag;
        }
        match session_state {
            1 => SessionPhase::GettingInCar,
            3 => SessionPhase::Formation,
            4 => SessionPhase::Racing,
            5 => SessionPhase::Checkered,
            6 => SessionPhase::Finished,
            _ => SessionPhase::Unknown,
        }
    }
}

// ---------------------------------------------------------------------------
// Flag state
// ---------------------------------------------------------------------------

/// Flags relevant to the race engineer.
#[derive(Debug, Clone, Default)]
pub struct FlagState {
    /// True if there is a full-course caution / yellow flag active.
    pub yellow_sectors: [bool; 3],
    pub blue: bool,
    pub red: bool,
    pub debris: bool,
    /// Meatball flag: iRacing requires the player to pit for repairs
    /// (SessionFlags bit 0x00100000 = irsdk_repair).
    pub meatball: bool,
    pub player_under_yellow: bool,
    /// Simplified: player "sector" — always 0 for iRacing (no per-sector yellow).
    pub player_in_yellow_sector: bool,
    pub player_sector_idx: usize,
}

impl FlagState {
    pub fn from_iracing(session_flags: u32, player_car_flags: Option<u32>) -> Self {
        // iRacing SessionFlags bits (global)
        const YELLOW: u32 = 0x0008;
        const RED: u32 = 0x0010;
        const DEBRIS: u32 = 0x0040;
        const YELLOW_WAVING: u32 = 0x0100;
        const CAUTION: u32 = 0x4000;
        const CAUTION_WAVING: u32 = 0x8000;
        // "Driver's black flags" — set in global SessionFlags for the local player only
        const REPAIR: u32 = 0x00100000; // meatball flag
        // CarIdxSessionFlags carries the same irsdk_Flags bit layout as the
        // global field (irsdk_defines.h: blue=0x0020, debris=0x0040). The
        // previous values 0x0400/0x0800 were greenHeld/tenToGo and could
        // never represent blue/debris.
        const CAR_BLUE: u32 = 0x0020;
        const CAR_DEBRIS: u32 = 0x0040;

        let any_yellow = (session_flags & (YELLOW | YELLOW_WAVING | CAUTION | CAUTION_WAVING)) != 0;
        let any_red = (session_flags & RED) != 0;
        let any_debris = (session_flags & DEBRIS) != 0
            || player_car_flags.map_or(false, |f| (f & CAR_DEBRIS) != 0);
        let blue = player_car_flags.map_or(false, |f| (f & CAR_BLUE) != 0);
        let meatball = (session_flags & REPAIR) != 0;

        FlagState {
            yellow_sectors: [any_yellow, false, false],
            blue,
            red: any_red,
            debris: any_debris,
            meatball,
            player_under_yellow: any_yellow,
            player_in_yellow_sector: any_yellow,
            player_sector_idx: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Damage state (limited in iRacing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct DamageState {
    pub has_aero: bool,
    pub has_suspension: bool,
    pub overheating: bool,
    pub any_detached: bool,
    pub last_impact_magnitude: f64,
}

impl DamageState {
    pub fn from_iracing(engine_warnings: u32) -> Self {
        // EngineWarnings bit 0x01 = water temp warning (overheating proxy)
        // bit 0x40 = oil temp warning
        let overheating = (engine_warnings & 0x01) != 0 || (engine_warnings & 0x40) != 0;
        DamageState {
            has_aero: false,
            has_suspension: false,
            overheating,
            any_detached: false,
            last_impact_magnitude: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// PitState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PitState {
    #[default]
    None,
    InLane,
    InBox,
}

// ---------------------------------------------------------------------------
// EngineerState — the full snapshot passed to the rule engine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EngineerState {
    // Session
    pub session_type: SessionType,
    pub session_phase: SessionPhase,
    pub time_remaining: Option<Duration>,
    pub laps_remaining: Option<u32>,
    pub total_laps_driven: u32,

    // Player
    pub player_position: u32,
    pub player_class_position: u32,
    pub player_lap: u32,
    pub in_pit: bool,
    pub in_garage: bool,
    pub pit_state: PitState,
    pub pit_stops: u32,

    // Fuel
    pub fuel_remaining_l: f32,
    pub fuel_laps_left: f32,

    // Timing
    pub last_lap_time: Option<Duration>,
    pub best_lap_time_personal: Option<Duration>,
    /// True only on the tick where a new personal best was set (aggregator-tracked).
    pub personal_best_this_lap: bool,
    pub best_lap_time_session: Option<Duration>,
    pub current_lap_time: Duration,
    pub last_sector_deltas: [Option<f32>; 3],
    pub recent_lap_times: VecDeque<Duration>,

    // Gaps (race only; None in practice/qualifying)
    pub gap_ahead: Option<f32>,
    pub gap_behind: Option<f32>,

    // Flags
    pub active_flags: FlagState,

    // Tires — average per axle (simplified)
    pub tire_temps_c: [f32; 4],   // LF, RF, LR, RR average carcass temp
    pub tire_wear_pct: [f32; 4],  // LF, RF, LR, RR wear % (0–1)

    // Damage
    pub damage: DamageState,

    // Weather
    pub ambient_temp_c: f32,
    pub track_temp_c: f32,
    pub rain_intensity: f32,
    pub wind_speed_ms: Option<f32>,
    pub wind_dir_rad: Option<f32>,

    // Incident count (raw iRacing value, increments with each incident)
    pub incident_count: u32,
    /// Session incident limit (None = unlimited). Derived from WeekendOptions.IncidentLimit.
    pub incident_limit: Option<u32>,
    // Drive-through penalty currently active (CarIdxSessionFlags bit 0x10000000)
    pub has_drivethrough_penalty: bool,

    // Class rivals (None in practice with limited opponent data)
    pub class_rivals_avg_last_lap: Option<Duration>,
    pub class_rivals_min_best_lap: Option<Duration>,
    pub class_car_ahead_last_lap: Option<Duration>,
    pub class_car_behind_last_lap: Option<Duration>,
}

// ---------------------------------------------------------------------------
// StateAggregator — builds EngineerState from iRacing telemetry
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct StateAggregator {
    /// Player's lap count on the last tick (to detect lap completion).
    last_lap: i32,
    /// Last known fuel level (to compute consumption per lap).
    fuel_at_lap_start: f32,
    /// Recent fuel consumption per lap.
    fuel_per_lap_history: VecDeque<f32>,
    /// Rolling recent lap times for pace analysis.
    recent_lap_times: VecDeque<Duration>,
    /// Session incident limit (None = unlimited). Set from YAML via `set_incident_limit`.
    incident_limit: Option<u32>,
    /// Best lap time tracked internally (avoids iRacing LapBestLapTime tick-delay quirks).
    tracked_best_lap: Option<Duration>,
}

impl StateAggregator {
    pub fn set_incident_limit(&mut self, limit: Option<u32>) {
        self.incident_limit = limit;
    }
}

impl StateAggregator {
    /// Build an `EngineerState` from the current telemetry + standings snapshots.
    pub fn build_state(
        &mut self,
        tel: &TelemetrySnapshot,
        standings: Option<&StandingsSnapshot>,
    ) -> EngineerState {
        // --- Session ---
        let session_type = standings
            .map(|s| SessionType::from_str(&s.session_type))
            .unwrap_or(SessionType::Unknown);

        let session_phase =
            SessionPhase::from_iracing(tel.session_state, tel.session_flags);

        // iRacing reports 32767 for "unlimited" laps
        let laps_remaining = tel
            .session_laps_remain
            .filter(|&l| l > 0 && l < 32767)
            .map(|l| l as u32);

        let time_remaining = tel
            .session_time_remain
            .filter(|&t| t > 0.0 && t < 604800.0) // < 1 week
            .map(|t| Duration::from_secs_f64(t));

        // --- Player ---
        let player_position = tel.player_position.max(0) as u32;
        let player_class_position = tel.player_class_position.max(0) as u32;
        let player_lap = tel.lap.max(0) as u32;

        // Lap completion detection for tracking recent times + fuel
        let mut personal_best_this_lap = false;
        if tel.lap > self.last_lap && self.last_lap >= 0 && tel.lap_last_time > 0.0 {
            let dur = Duration::from_secs_f32(tel.lap_last_time);
            self.recent_lap_times.push_back(dur);
            if self.recent_lap_times.len() > 8 {
                self.recent_lap_times.pop_front();
            }
            // Track personal best independently of iRacing's LapBestLapTime timing
            if self.tracked_best_lap.map_or(true, |pb| dur < pb) {
                self.tracked_best_lap = Some(dur);
                personal_best_this_lap = true;
            }
            // Fuel per lap
            if self.fuel_at_lap_start > 0.0 && tel.fuel_level <= self.fuel_at_lap_start {
                let used = self.fuel_at_lap_start - tel.fuel_level;
                if used > 0.05 {
                    self.fuel_per_lap_history.push_back(used);
                    if self.fuel_per_lap_history.len() > 5 {
                        self.fuel_per_lap_history.pop_front();
                    }
                }
            }
            self.fuel_at_lap_start = tel.fuel_level;
        }
        self.last_lap = tel.lap;

        // --- Fuel ---
        let fuel_remaining_l = tel.fuel_level;
        let fuel_laps_left = self.compute_fuel_laps(tel);

        // --- Timing ---
        let last_lap_time = if tel.lap_last_time > 0.0 {
            Some(Duration::from_secs_f32(tel.lap_last_time))
        } else {
            None
        };
        let best_lap_time_personal = if tel.lap_best_time > 0.0 {
            Some(Duration::from_secs_f32(tel.lap_best_time))
        } else {
            None
        };

        // Session-best in class from standings
        let player_car_idx = tel.player_car_idx;
        let player_class_id = standings.and_then(|s| {
            s.entries
                .iter()
                .find(|e| e.car_idx == player_car_idx)
                .map(|e| e.car_class_id)
        });

        let best_lap_time_session = player_class_id
            .and_then(|class_id| {
                standings.map(|s| {
                    s.entries
                        .iter()
                        .filter(|e| e.car_class_id == class_id && e.best_lap_time > 0.0)
                        .map(|e| e.best_lap_time)
                        .fold(f32::MAX, f32::min)
                })
            })
            .filter(|&t| t < f32::MAX && t > 0.0)
            .map(|t| Duration::from_secs_f32(t));

        // Sector deltas from last 3 sectors vs personal best
        let sector_deltas = [None, None, None]; // populated from sector_tracker externally if needed

        // --- Flags ---
        let active_flags = FlagState::from_iracing(
            tel.session_flags,
            tel.player_car_session_flags,
        );

        // --- Tires ---
        let tire_temps_c = [
            avg3(tel.tire_temp_lf),
            avg3(tel.tire_temp_rf),
            avg3(tel.tire_temp_lr),
            avg3(tel.tire_temp_rr),
        ];
        let tire_wear_pct = [
            avg3(tel.tire_wear_lf),
            avg3(tel.tire_wear_rf),
            avg3(tel.tire_wear_lr),
            avg3(tel.tire_wear_rr),
        ];

        // --- Damage ---
        let damage = DamageState::from_iracing(tel.engine_warnings);

        // --- Pit ---
        let in_pit = tel.on_pit_road;
        let in_garage = tel.is_in_garage;
        let pit_state = if in_pit { PitState::InLane } else { PitState::None };

        let pit_stops = standings
            .and_then(|s| s.entries.iter().find(|e| e.car_idx == player_car_idx))
            .map(|e| e.pit_stops)
            .unwrap_or(0);

        // --- Gaps (race only) ---
        let is_race = matches!(session_type, SessionType::Race);
        let (gap_ahead, gap_behind, class_car_ahead_last_lap, class_car_behind_last_lap) =
            compute_gaps(is_race, tel.player_car_idx, player_class_position, player_class_id, standings);

        // --- Class rivals ---
        let (class_rivals_avg_last_lap, class_rivals_min_best_lap) =
            compute_rival_pace(tel.player_car_idx, player_class_id, standings);

        // --- Weather ---
        let rain_intensity = tel.precipitation.unwrap_or(0.0)
            .max(if tel.track_wetness.unwrap_or(0) >= 3 { 0.3 } else { 0.0 });

        let incident_count = tel
            .player_car_my_incident_count
            .unwrap_or(0)
            .max(0) as u32;
        let has_drivethrough_penalty = tel.player_has_drivethrough_penalty;

        EngineerState {
            session_type,
            session_phase,
            time_remaining,
            laps_remaining,
            total_laps_driven: player_lap,
            player_position,
            player_class_position,
            player_lap,
            in_pit,
            in_garage,
            pit_state,
            pit_stops,
            fuel_remaining_l,
            fuel_laps_left,
            last_lap_time,
            best_lap_time_personal,
            personal_best_this_lap,
            best_lap_time_session,
            current_lap_time: Duration::from_secs_f32(tel.lap_current_time.max(0.0)),
            last_sector_deltas: sector_deltas,
            recent_lap_times: self.recent_lap_times.clone(),
            gap_ahead,
            gap_behind,
            active_flags,
            tire_temps_c,
            tire_wear_pct,
            damage,
            ambient_temp_c: tel.air_temp,
            track_temp_c: tel.track_temp,
            rain_intensity,
            wind_speed_ms: tel.wind_vel,
            wind_dir_rad: tel.wind_dir,
            incident_count,
            incident_limit: self.incident_limit,
            has_drivethrough_penalty,
            class_rivals_avg_last_lap,
            class_rivals_min_best_lap,
            class_car_ahead_last_lap,
            class_car_behind_last_lap,
        }
    }

    fn compute_fuel_laps(&self, tel: &TelemetrySnapshot) -> f32 {
        let fuel_level = tel.fuel_level;
        if fuel_level <= 0.0 {
            return 0.0;
        }

        // Prefer measured per-lap consumption
        if !self.fuel_per_lap_history.is_empty() {
            let avg: f32 = self.fuel_per_lap_history.iter().sum::<f32>()
                / self.fuel_per_lap_history.len() as f32;
            if avg > 0.01 {
                return fuel_level / avg;
            }
        }

        // Fallback: fuel_use_per_hour × best_lap_s / 3600
        let best_lap_s = tel.lap_best_time;
        if best_lap_s > 10.0 && tel.fuel_use_per_hour > 0.1 {
            let fuel_per_lap = tel.fuel_use_per_hour * best_lap_s / 3600.0;
            return fuel_level / fuel_per_lap;
        }

        0.0
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn avg3(arr: [f32; 3]) -> f32 {
    (arr[0] + arr[1] + arr[2]) / 3.0
}

fn compute_gaps(
    is_race: bool,
    player_car_idx: i32,
    player_class_pos: u32,
    player_class_id: Option<i32>,
    standings: Option<&StandingsSnapshot>,
) -> (Option<f32>, Option<f32>, Option<Duration>, Option<Duration>) {
    // Positional gap (time-to-leader) is only meaningful in a race.
    // Adjacent-car last-lap pace data is meaningful in any session.
    let Some(standings) = standings else {
        return (None, None, None, None);
    };
    let Some(class_id) = player_class_id else {
        return (None, None, None, None);
    };
    // Guard: player must be classified (class_pos > 0)
    if player_class_pos == 0 {
        return (None, None, None, None);
    }

    // Get sorted class entries by class position
    let mut class_entries: Vec<_> = standings
        .entries
        .iter()
        .filter(|e| e.car_class_id == class_id && e.class_position > 0)
        .collect();
    class_entries.sort_by_key(|e| e.class_position);

    // Positional gaps — race only (gap_to_leader is race-order lap-delta in practice)
    let (ahead, behind) = if is_race {
        let player_entry = standings.entries.iter().find(|e| e.car_idx == player_car_idx);
        let player_gap = player_entry.and_then(|e| e.gap_to_leader);

        let ahead = if player_class_pos > 1 {
            class_entries
                .iter()
                .find(|e| e.class_position == player_class_pos as i32 - 1)
                .and_then(|e| e.gap_to_leader)
                .and_then(|ahead_gap| player_gap.map(|pg| pg - ahead_gap))
                .filter(|&g| g >= 0.0)
        } else {
            None
        };

        let behind = class_entries
            .iter()
            .find(|e| e.class_position == player_class_pos as i32 + 1)
            .and_then(|e| e.gap_to_leader)
            .and_then(|behind_gap| player_gap.map(|pg| behind_gap - pg))
            .filter(|&g| g >= 0.0);

        (ahead, behind)
    } else {
        (None, None)
    };

    // Last lap times of adjacent class cars — available in all sessions
    let car_ahead_last = class_entries
        .iter()
        .find(|e| e.class_position == player_class_pos as i32 - 1)
        .filter(|e| e.last_lap_time > 0.0)
        .map(|e| Duration::from_secs_f32(e.last_lap_time));

    let car_behind_last = class_entries
        .iter()
        .find(|e| e.class_position == player_class_pos as i32 + 1)
        .filter(|e| e.last_lap_time > 0.0)
        .map(|e| Duration::from_secs_f32(e.last_lap_time));

    (ahead, behind, car_ahead_last, car_behind_last)
}

fn compute_rival_pace(
    player_car_idx: i32,
    player_class_id: Option<i32>,
    standings: Option<&StandingsSnapshot>,
) -> (Option<Duration>, Option<Duration>) {
    let Some(standings) = standings else {
        return (None, None);
    };
    let Some(class_id) = player_class_id else {
        return (None, None);
    };

    let rivals: Vec<_> = standings
        .entries
        .iter()
        .filter(|e| e.car_class_id == class_id && e.car_idx != player_car_idx)
        .collect();

    if rivals.is_empty() {
        return (None, None);
    }

    let valid_last: Vec<f32> = rivals
        .iter()
        .map(|e| e.last_lap_time)
        .filter(|&t| t > 0.0)
        .collect();

    let avg_last = if valid_last.is_empty() {
        None
    } else {
        let avg = valid_last.iter().sum::<f32>() / valid_last.len() as f32;
        Some(Duration::from_secs_f32(avg))
    };

    let min_best = rivals
        .iter()
        .map(|e| e.best_lap_time)
        .filter(|&t| t > 0.0)
        .fold(f32::MAX, f32::min);

    let min_best = if min_best < f32::MAX {
        Some(Duration::from_secs_f32(min_best))
    } else {
        None
    };

    (avg_last, min_best)
}
