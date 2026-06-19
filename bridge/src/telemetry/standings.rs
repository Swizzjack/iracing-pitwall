//! 4-Hz-Snapshot: CarIdx-basierte Standings inkl. berechneter Gaps.

use crate::error::Result;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::iracing_sdk::IRacingClient;
use crate::telemetry::finish_tracker::FinishTracker;
use crate::telemetry::p2p_tracker::{P2pAvailability, P2pTracker};
use crate::telemetry::pit_tracker::PitTracker;
use crate::telemetry::sector_tracker::SectorTracker;
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
    /// Theoretical session-best time per sector (minimum across all cars' personal bests).
    pub session_best_sectors: Vec<Option<f32>>,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct StandingEntry {
    pub car_idx: i32,
    pub position: i32,
    pub class_position: i32,
    pub car_class_id: i32,
    pub car_class_short_name: String,
    pub car_class_color: Option<i64>,
    pub manufacturer: Option<String>,
    pub user_name: String,
    pub car_number: String,
    pub irating: i32,
    pub safety_rating: String,
    pub lic_color: Option<i64>,
    pub lap: i32,
    pub lap_dist_pct: f32,
    pub last_lap_time: f32,
    pub best_lap_time: f32,
    /// Seconds behind the in-class leader. Race: `CarIdxF2Time`. Practice/Qualify:
    /// `driver.fastest_time − class_leader.fastest_time`. `None` = no valid data yet.
    pub gap_to_leader: Option<f32>,
    pub on_pit_road: bool,
    pub tire_compound: Option<i32>,
    /// Remaining push-to-pass seconds (decoded from `CarIdxP2P_Count`, which the
    /// SDK declares as `Int` but actually stores as raw Float32 bits × 10).
    /// `None` if the car has no P2P system / no valid value.
    pub p2p_remaining: Option<f32>,
    /// True while the driver is actively using push-to-pass (`CarIdxP2P_Status`).
    pub p2p_active: bool,
    /// Seconds remaining in the mandatory post-deactivation cooldown (e.g. the
    /// SF23's 100s P2P delay), or `None` if not currently on cooldown.
    pub p2p_cooldown: Option<f32>,
    /// Whether P2P is a real countdown (`Limited`), unlimited (`~999`, e.g.
    /// Practice) or unavailable (`0`, e.g. Qualifying). Drives N/A vs ∞ display.
    pub p2p_availability: P2pAvailability,
    pub pit_stops: u32,
    pub last_pit_road_sec: Option<f32>,
    pub current_pit_road_sec: Option<f32>,
    /// Sector times from the most recently completed clean lap.
    pub last_sector_times: Vec<f32>,
    /// Personal-best sector time per sector. None until that sector has been completed cleanly.
    pub best_sector_times: Vec<Option<f32>>,
    /// Sector times completed so far in the current (still-running) lap.
    pub current_lap_sectors: Vec<f32>,
    /// True once the car has crossed the S/F line under the checkered flag.
    pub finished: bool,
    /// Estimated time (s) from the S/F line to the car's current track position
    /// (`CarIdxEstTime`). Backend-only input for live race-gap computation —
    /// `CarIdxF2Time` refreshes only at the start/finish line during races.
    #[serde(skip)]
    #[ts(skip)]
    pub est_time: Option<f32>,
}

/// Whether a finalized `ResultsPositions` entry represents a car that did NOT
/// finish the race (retired, disconnected, disqualified, …). iRacing reports a
/// classified finisher with `ReasonOutStr == "Running"`; any other non-empty
/// reason means the car left the race early. An empty/unknown reason is treated
/// as a finisher so we fall through to the normal laps-down/time gap rather than
/// blanking a legitimate gap.
fn is_dnf(res: &crate::iracing_sdk::types::ResultPosition) -> bool {
    let reason = res.reason_out_str.trim();
    !reason.is_empty() && !reason.eq_ignore_ascii_case("Running")
}

impl StandingsSnapshot {
    /// Builds a standings snapshot by merging live CarIdx telemetry arrays with
    /// YAML DriverInfo (names/numbers) and ResultsPositions (gap calculation).
    pub fn build(
        client: &IRacingClient,
        yaml: &SessionInfoYaml,
        pit_tracker: &PitTracker,
        sector_tracker: &SectorTracker,
        p2p_tracker: &P2pTracker,
        finish_tracker: &mut FinishTracker,
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
        // F2Time = seconds behind in-class leader during a race; absent in older
        // builds and meaningless outside race sessions, so it's optional.
        let f2_times = client.get_f32_array("CarIdxF2Time").ok();
        // EstTime = per-car estimated time from S/F to its current position,
        // updated continuously (unlike F2Time) — used for live gaps.
        let est_times = client.get_f32_array("CarIdxEstTime").ok();
        // Tire compound index per car; absent in some builds/sessions.
        let tire_compounds = client.get_i32_array("CarIdxTireCompound").ok();
        // P2P remaining/active/cooldown are derived purely from the timer by the
        // P2pTracker (CarIdxP2P_Status is unreliable in live sessions) — see
        // p2p_tracker.rs and the project_p2p_encoding memory.

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
                // Always drop pace car. Drop spectators only if they have no
                // classified result — after a race, iRacing may re-flag a DNF
                // driver as spectator while their ResultsPositions entry remains.
                if driver.car_is_pace_car != 0 {
                    return None;
                }
                let res = results_map.get(&driver.car_idx);
                if driver.is_spectator != 0 && res.is_none() {
                    return None;
                }

                // Return frozen entry immediately if this car has already finished.
                if let Some(frozen) = finish_tracker.frozen_entry(driver.car_idx) {
                    return Some(frozen.clone());
                }

                let idx = driver.car_idx as usize;
                let pos = *positions.get(idx).unwrap_or(&0);
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

                // CarIdxLap < 0 means the driver has left the server — only then fall
                // back to stale ResultsPositions data. Active drivers keep their live
                // value even when it is -1 (invalid lap); the frontend renders -1 as '—'.
                let live_lap = *laps.get(idx).unwrap_or(&-1);
                let driver_departed = live_lap < 0;

                let live_last = *last_lap_times.get(idx).unwrap_or(&-1.0);
                let last_lap_time = if live_last > 0.0 {
                    live_last
                } else if driver_departed {
                    res.filter(|r| r.last_time > 0.0)
                        .map(|r| r.last_time as f32)
                        .unwrap_or(-1.0)
                } else {
                    -1.0
                };

                let live_best = *best_lap_times.get(idx).unwrap_or(&-1.0);
                let best_lap_time = if live_best > 0.0 {
                    live_best
                } else if driver_departed {
                    res.filter(|r| r.fastest_time > 0.0)
                        .map(|r| r.fastest_time as f32)
                        .unwrap_or(-1.0)
                } else {
                    -1.0
                };

                let manufacturer = driver
                    .car_screen_name_short
                    .as_ref()
                    .and_then(|s| s.split_whitespace().next().map(str::to_string));

                let pit = pit_tracker.get(driver.car_idx);
                let sectors = sector_tracker.get(driver.car_idx);
                let live_entry = StandingEntry {
                    car_idx: driver.car_idx,
                    position: pos,
                    class_position: class_pos,
                    car_class_id: driver.car_class_id,
                    car_class_short_name: driver.car_class_short_name.clone().unwrap_or_default(),
                    car_class_color: driver.car_class_color,
                    manufacturer,
                    user_name: driver.user_name.clone(),
                    car_number: driver.car_number.clone(),
                    irating: driver.irating,
                    safety_rating: driver.lic_string.clone(),
                    lic_color: driver.lic_color,
                    lap: *laps.get(idx).unwrap_or(&0),
                    lap_dist_pct: *lap_dist_pcts.get(idx).unwrap_or(&0.0),
                    last_lap_time,
                    best_lap_time,
                    gap_to_leader,
                    on_pit_road: *on_pit.get(idx).unwrap_or(&false),
                    tire_compound: tire_compounds.as_ref().and_then(|arr| arr.get(idx).copied())
                        .filter(|&c| c >= 0),
                    p2p_remaining: p2p_tracker.remaining(driver.car_idx),
                    p2p_active: p2p_tracker.is_active(driver.car_idx),
                    p2p_cooldown: p2p_tracker.cooldown_remaining(driver.car_idx),
                    p2p_availability: p2p_tracker.availability(driver.car_idx),
                    pit_stops: pit.map_or(0, |p| p.pit_stops),
                    last_pit_road_sec: pit.and_then(|p| p.last_pit_road_sec),
                    current_pit_road_sec: pit.and_then(|p| p.current_pit_road_sec),
                    last_sector_times: sectors.map(|s| s.last_sectors.clone()).unwrap_or_default(),
                    best_sector_times: sectors.map(|s| s.personal_best.clone()).unwrap_or_default(),
                    current_lap_sectors: sectors.map(|s| s.current_lap_sectors.clone()).unwrap_or_default(),
                    finished: false,
                    // LapDistPct is -1 while the car is not in the world (garage,
                    // not loaded) — EstTime carries no usable signal then.
                    est_time: est_times
                        .as_ref()
                        .and_then(|arr| arr.get(idx).copied())
                        .filter(|&t| t >= 0.0)
                        .filter(|_| *lap_dist_pcts.get(idx).unwrap_or(&-1.0) >= 0.0),
                };

                // Freeze on first tick where checkered is set AND this car's lap counter
                // incremented (= the car just crossed the S/F line under the checkered flag).
                if finish_tracker.checkered() && finish_tracker.has_incremented(driver.car_idx) {
                    let mut finished_entry = live_entry.clone();
                    finished_entry.finished = true;
                    finish_tracker.freeze_if_new(driver.car_idx, finished_entry);
                    // Return the now-frozen copy.
                    return Some(finish_tracker.frozen_entry(driver.car_idx).unwrap().clone());
                }

                Some(live_entry)
            })
            .collect();

        // Once the race is *fully over* (SessionState == CoolDown, latched by the
        // finish_tracker), the live CarIdx* arrays are unreliable: parked/retired cars
        // stop updating and others get renumbered, producing duplicate positions, a
        // scrambled order, and gaps that drift from the official result. ResultsPositions
        // is then the authoritative, final classification — overwrite both frozen and
        // live entries with it so the standings exactly match iRacing's result screen.
        //
        // Gate on session_finished(), NOT checkered(): the checkered flag is shown when
        // the *leader* finishes while everyone else (incl. the player) is still on their
        // last lap, and at that point ResultsPositions holds the previous lap's order
        // with no final times yet — overwriting there froze a stale "last-lap" order and
        // wiped every gap to "—". CoolDown is the first point where the result is final.
        //
        // Note on indexing: ResultsPositions `Position` is 1-based (matches CarIdxPosition),
        // but `ClassPosition` is 0-based (class winner = 0) — add 1 to match the 1-based
        // live CarIdxClassPosition the rest of the UI expects.
        if is_race && finish_tracker.session_finished() {
            // Per-class winner (lowest ResultsPositions class_position) — anchor for the
            // race gap. Picking the minimum is robust to the 0-/1-based ambiguity above.
            let mut class_leader: HashMap<i32, &crate::iracing_sdk::types::ResultPosition> =
                HashMap::new();
            for entry in &entries {
                if let Some(&res) = results_map.get(&entry.car_idx) {
                    class_leader
                        .entry(entry.car_class_id)
                        .and_modify(|cur| {
                            if res.class_position < cur.class_position {
                                *cur = res;
                            }
                        })
                        .or_insert(res);
                }
            }

            for entry in &mut entries {
                if let Some(&res) = results_map.get(&entry.car_idx) {
                    entry.position = res.position;
                    entry.class_position = res.class_position + 1;

                    // Race gap from the official result, relative to the in-class winner:
                    //   winner            → 0      (frontend renders "—")
                    //   retired / DNF     → None   ("—") — didn't finish, no meaningful gap
                    //   laps down         → -N     (frontend renders "+NL")
                    //   same lap          → time delta in seconds ("+X.XXX")
                    // ResultsPositions `Time` is the gap to the OVERALL leader in seconds
                    // (the overall leader is exactly 0.0), so the in-class gap is
                    // `res.time - class_leader.time` — for a single class this is just
                    // `res.time`; for multiclass it subtracts the class leader's own
                    // deficit to the overall leader.
                    if let Some(leader) = class_leader.get(&entry.car_class_id) {
                        entry.gap_to_leader = if res.car_idx == leader.car_idx {
                            Some(0.0)
                        } else if is_dnf(res) {
                            None
                        } else if res.laps_complete < leader.laps_complete {
                            Some(-((leader.laps_complete - res.laps_complete) as f32))
                        } else {
                            Some((res.time - leader.time) as f32)
                        };
                    }
                }
            }
        }

        entries.sort_unstable_by(|a, b| {
            let a_unclass = a.position == 0;
            let b_unclass = b.position == 0;
            a_unclass
                .cmp(&b_unclass)
                .then(a.position.cmp(&b.position))
                .then(a.user_name.cmp(&b.user_name))
        });

        Ok(Self {
            session_num,
            session_type,
            entries,
            session_best_sectors: sector_tracker.session_best_sectors(),
        })
    }
}
