//! 4-Hz-Snapshot: Standings aus offiziellen iRacing-`ResultsPositions`.
//!
//! Reihenfolge und Gaps stammen ausschliesslich aus `ResultsPositions` (Session-
//! YAML), das iRacing 1x pro Runde am S/F-Punkt aktualisiert und – anders als die
//! `CarIdx*`-Live-Arrays – bei Pause/Box/Replay-Scrubbing NICHT auf 0 setzt. Die
//! Live-Arrays dienen nur noch für Felder ohne offizielles Pendant (Pit, Reifen,
//! Track-Position) und als Fallback in der Vor-Scoring-Phase (Grid/Out-Lap, bevor
//! ein erstes offizielles Ergebnis existiert). P2P, PIT und Sektoren bleiben live.

use crate::error::Result;
use crate::iracing_sdk::types::{ResultPosition, SessionInfoYaml};
use crate::iracing_sdk::IRacingClient;
use crate::telemetry::finish_tracker::FinishTracker;
use crate::telemetry::gap_tracker::GapTracker;
use crate::telemetry::p2p_tracker::{P2pAvailability, P2pTracker};
use crate::telemetry::pit_tracker::PitTracker;
use crate::telemetry::sector_tracker::SectorTracker;
use serde::Serialize;
use std::collections::HashMap;
use ts_rs::TS;

/// Which "lifecycle" the standings are currently in, surfaced to the UI as a
/// header badge so the user knows whether the order is still updating live or is
/// the locked-in official result.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "lowercase")]
pub enum StandingsMode {
    /// Order is updating from live telemetry.
    Live,
    /// Race only: checkered flag is out, cars are still completing.
    Finishing,
    /// Race only: session reached CoolDown, `ResultsPositions` is the final
    /// official classification.
    Final,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct StandingsSnapshot {
    pub session_num: i32,
    pub session_type: String,
    pub mode: StandingsMode,
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
    /// Gap to the in-class leader, from the official `ResultsPositions`.
    /// Race: `0` for the leader, `res.time − class_leader.time` (seconds) for a
    /// car on the lead lap, a negative integer `−N` when N laps down (frontend
    /// renders "+NL"), `None` for a DNF/retired car or before the first scoring
    /// update. Practice/Qualify: `driver.fastest_time − class_leader.fastest_time`.
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
}

/// Whether a `ResultsPositions` entry represents a car that did NOT finish the
/// race (retired, disconnected, disqualified, …). iRacing reports a classified
/// finisher with `ReasonOutStr == "Running"`; any other non-empty reason means
/// the car left the race early. An empty/unknown reason is treated as a finisher
/// so we fall through to the normal laps-down/time gap rather than blanking a
/// legitimate gap.
fn is_dnf(res: &ResultPosition) -> bool {
    let reason = res.reason_out_str.trim();
    !reason.is_empty() && !reason.eq_ignore_ascii_case("Running")
}

impl StandingsSnapshot {
    /// Builds a standings snapshot by merging the official `ResultsPositions`
    /// (position, class position, gap) with YAML DriverInfo (names/numbers/class)
    /// and the live trackers for P2P, pit and sectors.
    pub fn build(
        client: &IRacingClient,
        yaml: &SessionInfoYaml,
        pit_tracker: &PitTracker,
        sector_tracker: &SectorTracker,
        p2p_tracker: &P2pTracker,
        finish_tracker: &FinishTracker,
        gap_tracker: &mut GapTracker,
    ) -> Result<Self> {
        let session_num = client.get_i32("SessionNum")?;
        gap_tracker.reset_if_session_changed(session_num);

        let current_session = yaml
            .session_info
            .sessions
            .iter()
            .find(|s| s.session_num == session_num);

        let session_type = current_session
            .map(|s| s.session_type.clone())
            .unwrap_or_default();

        // Live CarIdx arrays — only for fields with no official ResultsPositions
        // equivalent (pit road, tire, track position) and as the pre-scoring
        // fallback for position/lap times (grid / out-lap before a first result).
        let positions = client.get_i32_array("CarIdxPosition")?;
        let class_positions = client.get_i32_array("CarIdxClassPosition")?;
        let laps = client.get_i32_array("CarIdxLap")?;
        let lap_dist_pcts = client.get_f32_array("CarIdxLapDistPct")?;
        let last_lap_times = client.get_f32_array("CarIdxLastLapTime")?;
        let best_lap_times = client.get_f32_array("CarIdxBestLapTime")?;
        let on_pit = client.get_bool_array("CarIdxOnPitRoad")?;
        // Tire compound index per car; absent in some builds/sessions.
        let tire_compounds = client.get_i32_array("CarIdxTireCompound").ok();
        // P2P remaining/active/cooldown are derived purely from the timer by the
        // P2pTracker (CarIdxP2P_Status is unreliable in live sessions) — see
        // p2p_tracker.rs and the project_p2p_encoding memory.

        // car_idx → ResultPosition: the authoritative, pause-proof classification.
        let results_map: HashMap<i32, &ResultPosition> = current_session
            .and_then(|s| s.results_positions.as_ref())
            .map(|rp| rp.iter().map(|r| (r.car_idx, r)).collect())
            .unwrap_or_default();

        let is_race = session_type.eq_ignore_ascii_case("Race");

        // Per-class leader, anchor for the gap. Both maps join via
        // driver.car_class_id since ResultPosition carries no class id.
        //   Race:     the entry with the lowest class_position (class winner).
        //   Non-race: min(fastest_time) — gaps are a per-class best-lap delta.
        let mut class_leader: HashMap<i32, &ResultPosition> = HashMap::new();
        let mut class_leader_fastest: HashMap<i32, f64> = HashMap::new();
        for driver in &yaml.driver_info.drivers {
            if let Some(&res) = results_map.get(&driver.car_idx) {
                class_leader
                    .entry(driver.car_class_id)
                    .and_modify(|cur| {
                        if res.class_position < cur.class_position {
                            *cur = res;
                        }
                    })
                    .or_insert(res);
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

        let mut entries: Vec<StandingEntry> = yaml
            .driver_info
            .drivers
            .iter()
            .filter_map(|driver| {
                // Always drop the pace car. Drop spectators only if they have no
                // classified result — after a race iRacing may re-flag a DNF
                // driver as spectator while their ResultsPositions entry remains.
                if driver.car_is_pace_car != 0 {
                    return None;
                }
                let res = results_map.get(&driver.car_idx).copied();
                if driver.is_spectator != 0 && res.is_none() {
                    return None;
                }

                let idx = driver.car_idx as usize;

                // Position/class position from the official result (pause-proof).
                // Fall back to the live arrays only before the first scoring
                // update, when no ResultsPositions entry exists yet (grid /
                // out-lap). ResultsPositions `Position` is 1-based (matches
                // CarIdxPosition); `ClassPosition` is 0-based (winner = 0) → +1.
                let (position, class_position) = match res {
                    Some(r) if r.position > 0 => (r.position, r.class_position + 1),
                    _ => (
                        *positions.get(idx).unwrap_or(&0),
                        *class_positions.get(idx).unwrap_or(&0),
                    ),
                };

                // Gap to the in-class leader, entirely from ResultsPositions.
                let gap_to_leader: Option<f32> = if is_race {
                    match (res, class_leader.get(&driver.car_class_id).copied()) {
                        (Some(r), Some(leader)) => {
                            if r.car_idx == leader.car_idx {
                                Some(0.0)
                            } else if is_dnf(r) {
                                None
                            } else {
                                // Both the laps-down marker and the time gap go
                                // through the GapTracker, which holds the last
                                // value until this car crosses S/F again — so the
                                // leader starting a new lap no longer flips
                                // everyone to "+1L" mid-lap. `time` is the gap to
                                // the OVERALL leader; subtract the class leader's
                                // own deficit for multiclass.
                                let lap_delta = leader.laps_complete - r.laps_complete;
                                let time_gap = (r.time - leader.time) as f32;
                                Some(gap_tracker.resolve(
                                    r.car_idx,
                                    r.laps_complete,
                                    lap_delta,
                                    time_gap,
                                ))
                            }
                        }
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

                // CarIdxLap < 0 means the driver has left the server — only then
                // fall back to stale ResultsPositions data. Active drivers keep
                // their live value even when -1 (invalid lap); the frontend
                // renders -1 as '—'.
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
                Some(StandingEntry {
                    car_idx: driver.car_idx,
                    position,
                    class_position,
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
                    tire_compound: tire_compounds
                        .as_ref()
                        .and_then(|arr| arr.get(idx).copied())
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
                    current_lap_sectors: sectors
                        .map(|s| s.current_lap_sectors.clone())
                        .unwrap_or_default(),
                })
            })
            .collect();

        entries.sort_unstable_by(|a, b| {
            let a_unclass = a.position == 0;
            let b_unclass = b.position == 0;
            a_unclass
                .cmp(&b_unclass)
                .then(a.position.cmp(&b.position))
                .then(a.user_name.cmp(&b.user_name))
        });

        // Race lifecycle for the header badge. Mirrors the two finish stages:
        // CoolDown (session_finished) = locked official result; checkered-but-not-
        // CoolDown = cars still finishing. Non-race sessions are always "live".
        let mode = if is_race && finish_tracker.session_finished() {
            StandingsMode::Final
        } else if is_race && finish_tracker.checkered() {
            StandingsMode::Finishing
        } else {
            StandingsMode::Live
        };

        Ok(Self {
            session_num,
            session_type,
            mode,
            entries,
            session_best_sectors: sector_tracker.session_best_sectors(),
        })
    }
}
