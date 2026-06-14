//! EngineerState — a 10 Hz snapshot derived from iRacing telemetry.
//!
//! `StateAggregator::build_state()` converts `TelemetrySnapshot` +
//! `StandingsSnapshot` into the flat struct the rule engine operates on.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::telemetry::standings::StandingEntry;
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
// Part of the rule-author state surface: some fields have no consuming rule
// yet (e.g. per-sector yellows aren't available in iRacing V1).
#[allow(dead_code)]
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
            || player_car_flags.is_some_and(|f| (f & CAR_DEBRIS) != 0);
        let blue = player_car_flags.is_some_and(|f| (f & CAR_BLUE) != 0);
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

// V1 placeholder surface: only `overheating` carries a real signal so far;
// the remaining fields wait for damage telemetry rules.
#[allow(dead_code)]
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
    /// Not yet derivable from telemetry (needs pit-box position detection).
    #[allow(dead_code)]
    InBox,
}

// ---------------------------------------------------------------------------
// EngineerState — the full snapshot passed to the rule engine
// ---------------------------------------------------------------------------

// State surface for rule authors — fields without a consuming rule yet are
// kept deliberately (they're populated and cost nothing per tick).
#[allow(dead_code)]
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
    /// True only on the tick where the just-completed lap's time was recorded.
    /// This is ~1–2 s after the line: iRacing publishes `LapLastLapTime` with a
    /// delay, so lap-gated rules must key off this flag — not the lap counter —
    /// to see the fresh lap time instead of the previous lap's.
    pub lap_just_completed: bool,
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

/// Armed when the lap counter increments; resolved once `LapLastLapTime`
/// changes (iRacing publishes the new time ~1–2 s after the line).
struct PendingLapTime {
    /// `LapLastLapTime` value seen at the increment tick — still the lap
    /// before's time until iRacing updates it.
    value_at_crossing: f32,
    since: Instant,
}

#[derive(Default)]
pub struct StateAggregator {
    /// Player's lap count on the last tick (to detect lap completion).
    last_lap: i32,
    /// In-flight lap completion waiting for `LapLastLapTime` to update.
    pending_lap_time: Option<PendingLapTime>,
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
            .map(Duration::from_secs_f64);

        // --- Player ---
        let player_position = tel.player_position.max(0) as u32;
        let player_class_position = tel.player_class_position.max(0) as u32;
        let player_lap = tel.lap.max(0) as u32;

        // Lap completion detection. The lap counter increments at the line, but
        // iRacing publishes the lap's time (LapLastLapTime) only ~1–2 s later —
        // latching the value at the increment tick would record the *previous*
        // lap's time and shift every lap-time announcement one lap late. The
        // increment therefore only arms a pending latch (and handles fuel,
        // which IS current at the crossing); the latch resolves once
        // LapLastLapTime changes.
        if tel.lap == self.last_lap + 1 && self.last_lap > 0 {
            self.pending_lap_time = Some(PendingLapTime {
                value_at_crossing: tel.lap_last_time,
                since: Instant::now(),
            });
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
        } else if tel.lap != self.last_lap {
            // Jump or reset (session change, tow, joining mid-session): no lap
            // was completed under our observation — rebaseline silently.
            self.pending_lap_time = None;
            self.fuel_at_lap_start = tel.fuel_level;
        }
        self.last_lap = tel.lap;

        let mut personal_best_this_lap = false;
        let mut lap_just_completed = false;
        if let Some(pending) = &self.pending_lap_time {
            let updated =
                tel.lap_last_time > 0.0 && tel.lap_last_time != pending.value_at_crossing;
            // Two consecutive identical lap times never change the value; the
            // timeout (far beyond iRacing's publish delay) records them anyway.
            let timed_out =
                tel.lap_last_time > 0.0 && pending.since.elapsed() > Duration::from_secs(10);
            if updated || timed_out {
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
                lap_just_completed = true;
                self.pending_lap_time = None;
            }
        }

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
            .map(Duration::from_secs_f32);

        // Sector deltas: sectors completed so far this lap vs personal best
        // (positive = slower). Sourced from the standings' sector tracker data.
        let sector_deltas = compute_sector_deltas(player_car_idx, standings);

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
            lap_just_completed,
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

/// Delta per completed sector of the current lap vs the driver's personal
/// best for that sector (positive = slower). Covers the first three sectors;
/// `None` where the sector hasn't been completed or no PB exists yet.
fn compute_sector_deltas(
    player_car_idx: i32,
    standings: Option<&StandingsSnapshot>,
) -> [Option<f32>; 3] {
    let mut out = [None, None, None];
    let Some(entry) = standings
        .and_then(|s| s.entries.iter().find(|e| e.car_idx == player_car_idx))
    else {
        return out;
    };
    for (i, slot) in out.iter_mut().enumerate() {
        if let (Some(&cur), Some(Some(pb))) = (
            entry.current_lap_sectors.get(i),
            entry.best_sector_times.get(i),
        ) {
            if cur > 0.0 && *pb > 0.0 {
                *slot = Some(cur - *pb);
            }
        }
    }
    out
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

    // Positional gaps — race only (gap_to_leader is race-order lap-delta in practice).
    // Primary source is the live EstTime-based gap; the F2Time delta is kept as
    // fallback, but F2Time only refreshes at the start/finish line in races, so
    // on its own it repeats the same gap for a whole lap.
    let (ahead, behind) = if is_race {
        let player_entry = standings.entries.iter().find(|e| e.car_idx == player_car_idx);
        let player_gap = player_entry.and_then(|e| e.gap_to_leader);

        let ahead_entry = class_entries
            .iter()
            .find(|e| e.class_position == player_class_pos as i32 - 1)
            .filter(|_| player_class_pos > 1);
        let behind_entry = class_entries
            .iter()
            .find(|e| e.class_position == player_class_pos as i32 + 1);

        let ahead = ahead_entry
            .and_then(|a| {
                live_gap(a, player_entry?).or_else(|| {
                    a.gap_to_leader
                        .and_then(|ahead_gap| player_gap.map(|pg| pg - ahead_gap))
                })
            })
            .filter(|&g| g >= 0.0);

        let behind = behind_entry
            .and_then(|b| {
                live_gap(player_entry?, b).or_else(|| {
                    b.gap_to_leader
                        .and_then(|behind_gap| player_gap.map(|pg| behind_gap - pg))
                })
            })
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

/// Live gap in seconds between two cars, derived from `CarIdxEstTime`.
///
/// EstTime is each car's modeled time from the S/F line to its current track
/// position and updates continuously, but resets to 0 at the line. The number
/// of whole-lap wraps between the two cars is recovered from the lap-counter +
/// track-fraction progress difference, scaled by a reference lap time.
fn live_gap(leading: &StandingEntry, trailing: &StandingEntry) -> Option<f32> {
    let est_lead = leading.est_time?;
    let est_trail = trailing.est_time?;
    if leading.lap < 0 || trailing.lap < 0 {
        return None;
    }
    let ref_lap = [
        trailing.best_lap_time,
        trailing.last_lap_time,
        leading.best_lap_time,
        leading.last_lap_time,
    ]
    .into_iter()
    .find(|&t| t > 0.0)?;

    let progress_delta = (leading.lap as f32 + leading.lap_dist_pct)
        - (trailing.lap as f32 + trailing.lap_dist_pct);
    let raw = est_lead - est_trail;
    let wraps = ((progress_delta * ref_lap - raw) / ref_lap).round();
    Some(raw + wraps * ref_lap)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tel(lap: i32, lap_last_time: f32) -> TelemetrySnapshot {
        TelemetrySnapshot {
            lap,
            lap_last_time,
            fuel_level: 30.0,
            ..Default::default()
        }
    }

    /// iRacing publishes LapLastLapTime ~1–2 s after the lap counter
    /// increments — the lap must be recorded when the value updates, not at
    /// the increment tick (which still holds the previous lap's time).
    #[test]
    fn lap_time_recorded_when_value_updates_not_at_increment() {
        let mut agg = StateAggregator::default();
        let s = agg.build_state(&tel(1, 0.0), None);
        assert!(!s.lap_just_completed);

        // Crossed the line: counter increments, time not yet published
        let s = agg.build_state(&tel(2, 0.0), None);
        assert!(!s.lap_just_completed);
        assert!(s.recent_lap_times.is_empty());

        // Time published a moment later → recorded now, flagged as PB
        let s = agg.build_state(&tel(2, 92.5), None);
        assert!(s.lap_just_completed);
        assert!(s.personal_best_this_lap);
        assert_eq!(s.recent_lap_times.len(), 1);

        // Next lap, slower: recorded, but no PB
        let s = agg.build_state(&tel(3, 92.5), None);
        assert!(!s.lap_just_completed);
        let s = agg.build_state(&tel(3, 95.0), None);
        assert!(s.lap_just_completed);
        assert!(!s.personal_best_this_lap);
        assert_eq!(s.recent_lap_times.len(), 2);
    }

    /// Joining a session mid-race must not record the stale LapLastLapTime
    /// (it belongs to a lap we never observed).
    #[test]
    fn joining_mid_session_records_no_stale_lap() {
        let mut agg = StateAggregator::default();
        let s = agg.build_state(&tel(8, 91.0), None);
        assert!(!s.lap_just_completed);
        let s = agg.build_state(&tel(8, 91.0), None);
        assert!(!s.lap_just_completed);
        assert!(s.recent_lap_times.is_empty());

        // The first fully observed lap is recorded normally
        let s = agg.build_state(&tel(9, 91.0), None);
        assert!(!s.lap_just_completed);
        let s = agg.build_state(&tel(9, 90.3), None);
        assert!(s.lap_just_completed);
        assert_eq!(s.recent_lap_times.len(), 1);
    }

    fn entry(lap: i32, pct: f32, est: f32) -> StandingEntry {
        StandingEntry {
            lap,
            lap_dist_pct: pct,
            est_time: Some(est),
            best_lap_time: 90.0,
            ..Default::default()
        }
    }

    #[test]
    fn live_gap_same_lap() {
        let lead = entry(10, 0.65, 65.0);
        let trail = entry(10, 0.60, 60.0);
        assert!((live_gap(&lead, &trail).unwrap() - 5.0).abs() < 1e-3);
    }

    #[test]
    fn live_gap_across_start_finish() {
        // Leader just crossed the line (EstTime reset), trailer hasn't yet
        let lead = entry(11, 0.02, 2.0);
        let trail = entry(10, 0.97, 88.0);
        assert!((live_gap(&lead, &trail).unwrap() - 4.0).abs() < 1e-3);
    }

    #[test]
    fn live_gap_full_lap_ahead() {
        let lead = entry(11, 0.5, 45.0);
        let trail = entry(10, 0.5, 45.0);
        assert!((live_gap(&lead, &trail).unwrap() - 90.0).abs() < 1e-3);
    }

    #[test]
    fn live_gap_requires_est_time() {
        let lead = StandingEntry { est_time: None, ..entry(10, 0.5, 0.0) };
        let trail = entry(10, 0.4, 40.0);
        assert!(live_gap(&lead, &trail).is_none());
    }
}
