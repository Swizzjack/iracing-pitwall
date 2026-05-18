//! Per-car sector-time tracking at 60 Hz from CarIdxLapDistPct + SessionTime.

use crate::error::Result;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::iracing_sdk::IRacingClient;
use std::collections::HashMap;

/// Data emitted for every car that crosses the S/F line.
/// Drained via `drain_completed_laps()` and forwarded to the lap buffer.
#[derive(Debug, Clone)]
pub struct LapCompletion {
    pub car_idx: i32,
    /// 1-based lap number within this tracker's session lifetime.
    pub lap_num: i32,
    /// Elapsed time from the previous S/F crossing to this one (real wall-clock, not sector sum).
    pub lap_time_sec: Option<f32>,
    /// Sector times accumulated during this lap (may be partial/empty for invalid laps).
    pub sectors: Vec<f32>,
    /// True iff the lap was completed without pit/off-track events and all sectors are present.
    pub valid: bool,
    /// True iff the car entered pit road at any point during this lap.
    pub in_lap: bool,
    /// Session time at the S/F crossing that ended this lap.
    pub session_time: f64,
    pub air_temp: Option<f32>,
    pub track_temp: Option<f32>,
}

/// Sector timing data for one car, available after the first completed lap.
#[derive(Debug, Clone, Default)]
pub struct PerCarSectors {
    /// Sector times from the most recently completed (non-pit) lap, in order.
    pub last_sectors: Vec<f32>,
    /// Personal-best time per sector. None until that sector has been completed cleanly.
    pub personal_best: Vec<Option<f32>>,
}

#[derive(Debug)]
struct CarState {
    last_p: f32,
    last_t: f64,
    /// Which sector the car is currently in (index into sector_starts).
    sector_idx: usize,
    sector_started_at: f64,
    /// Session time at the S/F crossing that started the current lap.
    lap_started_at: f64,
    /// 1-based lap counter; 0 = no lap started yet.
    current_lap_num: i32,
    /// Accumulates sector times for the current lap; reset on S/F crossing or invalidation.
    current_lap_sectors: Vec<f32>,
    /// Whether the current lap is still clean (no pit/garage/off-world events).
    lap_valid: bool,
    /// True if the car entered pit road during the current lap.
    pit_in_during_lap: bool,
    output: PerCarSectors,
}

impl CarState {
    fn new(n_sectors: usize) -> Self {
        Self {
            last_p: -1.0,
            last_t: -1.0,
            sector_idx: 0,
            sector_started_at: 0.0,
            lap_started_at: 0.0,
            current_lap_num: 0,
            current_lap_sectors: Vec::new(),
            lap_valid: false,
            pit_in_during_lap: false,
            output: PerCarSectors {
                last_sectors: Vec::new(),
                personal_best: vec![None; n_sectors],
            },
        }
    }

    fn reset_lap(&mut self, t: f64, n_sectors: usize) {
        self.current_lap_num += 1;
        self.lap_started_at = t;
        self.sector_idx = 0;
        self.sector_started_at = t;
        self.current_lap_sectors = Vec::with_capacity(n_sectors);
        self.lap_valid = true;
        self.pit_in_during_lap = false;
    }

    /// Invalidate the current lap. `due_to_pit` records that a pit entry caused it.
    fn invalidate_lap(&mut self, due_to_pit: bool) {
        self.lap_valid = false;
        if due_to_pit {
            self.pit_in_during_lap = true;
        }
    }
}

/// Tracks sector crossings for every car at 60 Hz.
#[derive(Debug, Default)]
pub struct SectorTracker {
    cars: HashMap<i32, CarState>,
    /// Sorted sector start pcts, without 0.0 (S/F).
    sector_starts: Vec<f32>,
    last_session_num: Option<i32>,
    /// Lap completions since the last `drain_completed_laps()` call.
    pending_completions: Vec<LapCompletion>,
}

impl SectorTracker {
    /// Update sector tracking for all cars. Called every 60-Hz frame.
    pub fn update(&mut self, client: &IRacingClient, yaml: &SessionInfoYaml) -> Result<()> {
        let session_num = client.get_i32("SessionNum")?;
        if self.last_session_num != Some(session_num) {
            log::info!("sector_tracker: reset for session={session_num}");
            self.last_session_num = Some(session_num);
            self.cars.clear();
            self.pending_completions.clear();
        }

        let new_starts = yaml.sector_starts();
        if new_starts != self.sector_starts {
            self.sector_starts = new_starts;
            self.cars.clear();
            self.pending_completions.clear();
        }

        let n = self.sector_starts.len();
        if n == 0 {
            return Ok(());
        }

        let session_time = client.get_f64("SessionTime")?;
        let lap_dist_pcts = client.get_f32_array("CarIdxLapDistPct")?;
        let sdk_last_lap_times = client.get_f32_array("CarIdxLastLapTime").unwrap_or_default();
        let on_pit = client.get_bool_array("CarIdxOnPitRoad")?;
        let surfaces = client.get_i32_array("CarIdxTrackSurface").ok();
        let air_temp = client.get_f32("AirTemp").ok();
        let track_temp = client.get_f32("TrackTempCrew").ok();

        for (idx, &p) in lap_dist_pcts.iter().enumerate() {
            let car_idx = idx as i32;
            let surface = surfaces.as_ref().and_then(|a| a.get(idx).copied()).unwrap_or(0);
            let pit = *on_pit.get(idx).unwrap_or(&false);

            // Car not in world or negative pct → skip but don't reset.
            if surface == -1 || p < 0.0 {
                continue;
            }

            let state = self.cars.entry(car_idx).or_insert_with(|| CarState::new(n + 1));

            // Invalidate running lap if car goes to pit or leaves track.
            if pit || surface == 1 || surface == 2 {
                state.invalidate_lap(pit);
                state.last_p = p;
                state.last_t = session_time;
                continue;
            }

            let last_p = state.last_p;
            let last_t = state.last_t;

            // First ever sample — just arm.
            if last_p < 0.0 {
                state.last_p = p;
                state.last_t = session_time;
                continue;
            }

            // Detect S/F crossing (p wraps from >0.9 back to <0.1).
            let sf_cross = last_p > 0.9 && p < 0.1;

            if sf_cross {
                let t_sf = interpolate_time(last_p, last_t, p, session_time, 0.0);

                // Finish the last sector of the previous lap.
                if state.lap_valid && state.sector_idx == n {
                    let dur = (t_sf - state.sector_started_at) as f32;
                    if dur > 0.0 {
                        state.current_lap_sectors.push(dur);
                    }
                }
                // Commit the completed lap if we have exactly n+1 sector times.
                if state.lap_valid && state.current_lap_sectors.len() == n + 1 {
                    let times = state.current_lap_sectors.clone();
                    let pb = &mut state.output.personal_best;
                    if pb.len() < n + 1 {
                        pb.resize(n + 1, None);
                    }
                    for (i, &t) in times.iter().enumerate() {
                        pb[i] = Some(match pb[i] {
                            Some(best) if best <= t => best,
                            _ => t,
                        });
                    }
                    state.output.last_sectors = times;
                }

                // Emit a LapCompletion for every car that had a lap started.
                if state.current_lap_num > 0 {
                    // Prefer iRacing's authoritative CarIdxLastLapTime over our
                    // self-measured interpolation (which has ~60-Hz noise of ±4-7 ms).
                    // Fall back to interpolated time only when iRacing reports no value
                    // (e.g. off-track invalidation → SDK returns -1).
                    let sdk_last = sdk_last_lap_times.get(idx).copied().unwrap_or(-1.0);
                    let lap_time = if sdk_last > 0.0 {
                        Some(sdk_last)
                    } else if state.lap_started_at > 0.0 {
                        let t = (t_sf - state.lap_started_at) as f32;
                        if t > 0.0 { Some(t) } else { None }
                    } else {
                        None
                    };
                    let valid = state.lap_valid && state.current_lap_sectors.len() == n + 1;
                    self.pending_completions.push(LapCompletion {
                        car_idx,
                        lap_num: state.current_lap_num,
                        lap_time_sec: lap_time,
                        sectors: state.current_lap_sectors.clone(),
                        valid,
                        in_lap: state.pit_in_during_lap,
                        session_time: t_sf,
                        air_temp,
                        track_temp,
                    });
                }

                // Start fresh lap.
                state.reset_lap(t_sf, n + 1);
                state.last_p = p;
                state.last_t = session_time;
                continue;
            }

            // Not on an active, valid lap — skip sector crossing logic.
            if !state.lap_valid {
                state.last_p = p;
                state.last_t = session_time;
                continue;
            }

            // Check if we crossed any sector boundaries between last_p and p.
            let mut si = state.sector_idx;
            while si < n {
                let threshold = self.sector_starts[si];
                let crossed = last_p < threshold && p >= threshold;
                if !crossed {
                    break;
                }
                let t_cross = interpolate_time(last_p, last_t, p, session_time, threshold);
                let dur = (t_cross - state.sector_started_at) as f32;
                if dur > 0.0 {
                    state.current_lap_sectors.push(dur);
                }
                state.sector_started_at = t_cross;
                state.sector_idx = si + 1;
                si += 1;
            }

            state.last_p = p;
            state.last_t = session_time;
        }

        Ok(())
    }

    pub fn get(&self, car_idx: i32) -> Option<&PerCarSectors> {
        self.cars.get(&car_idx).map(|s| &s.output)
    }

    /// Drain all pending lap completions since the last call. Called from the 4-Hz tick.
    pub fn drain_completed_laps(&mut self) -> Vec<LapCompletion> {
        std::mem::take(&mut self.pending_completions)
    }

    /// Theoretical session best per sector: minimum across all cars' personal bests.
    pub fn session_best_sectors(&self) -> Vec<Option<f32>> {
        let n = self.sector_starts.len() + 1;
        if n == 1 {
            return vec![];
        }
        let mut best: Vec<Option<f32>> = vec![None; n];
        for state in self.cars.values() {
            for (i, &pb) in state.output.personal_best.iter().enumerate() {
                if let Some(t) = pb {
                    best[i] = Some(match best[i] {
                        Some(b) if b <= t => b,
                        _ => t,
                    });
                }
            }
        }
        best
    }
}

/// Linearly interpolate the time at which lap_dist_pct crossed `threshold`.
fn interpolate_time(p0: f32, t0: f64, p1: f32, t1: f64, threshold: f32) -> f64 {
    let dp = (p1 - p0) as f64;
    if dp.abs() < 1e-10 {
        return t0;
    }
    let alpha = ((threshold - p0) as f64 / dp).clamp(0.0, 1.0);
    t0 + alpha * (t1 - t0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker(starts: Vec<f32>) -> SectorTracker {
        SectorTracker {
            cars: HashMap::new(),
            sector_starts: starts,
            last_session_num: None,
            pending_completions: Vec::new(),
        }
    }

    /// Feed a sequence of (time, lapDistPct, onPit, surface) frames into state for car 0.
    /// Mirrors the S/F-crossing and sector logic from `update()`, including completion emission.
    fn feed(st: &mut SectorTracker, frames: &[(f64, f32, bool, i32)]) {
        let n = st.sector_starts.len();
        for &(t, p, pit, surface) in frames {
            if p < 0.0 || surface == -1 {
                continue;
            }
            let state = st.cars.entry(0).or_insert_with(|| CarState::new(n + 1));
            if pit || surface == 1 || surface == 2 {
                state.invalidate_lap(pit);
                state.last_p = p;
                state.last_t = t;
                continue;
            }
            let last_p = state.last_p;
            let last_t = state.last_t;
            if last_p < 0.0 {
                state.last_p = p;
                state.last_t = t;
                continue;
            }
            let sf_cross = last_p > 0.9 && p < 0.1;
            if sf_cross {
                let t_sf = interpolate_time(last_p, last_t, p, t, 0.0);
                if state.lap_valid && state.sector_idx == n {
                    let dur = (t_sf - state.sector_started_at) as f32;
                    if dur > 0.0 { state.current_lap_sectors.push(dur); }
                }
                if state.lap_valid && state.current_lap_sectors.len() == n + 1 {
                    let times = state.current_lap_sectors.clone();
                    let pb = &mut state.output.personal_best;
                    if pb.len() < n + 1 { pb.resize(n + 1, None); }
                    for (i, &sv) in times.iter().enumerate() {
                        pb[i] = Some(match pb[i] { Some(b) if b <= sv => b, _ => sv });
                    }
                    state.output.last_sectors = times;
                }
                // Emit completion (mirrors update()).
                if state.current_lap_num > 0 {
                    let lap_time = if state.lap_started_at > 0.0 {
                        let elapsed = (t_sf - state.lap_started_at) as f32;
                        if elapsed > 0.0 { Some(elapsed) } else { None }
                    } else {
                        None
                    };
                    let valid = state.lap_valid && state.current_lap_sectors.len() == n + 1;
                    st.pending_completions.push(LapCompletion {
                        car_idx: 0,
                        lap_num: state.current_lap_num,
                        lap_time_sec: lap_time,
                        sectors: state.current_lap_sectors.clone(),
                        valid,
                        in_lap: state.pit_in_during_lap,
                        session_time: t_sf,
                        air_temp: None,
                        track_temp: None,
                    });
                }
                state.reset_lap(t_sf, n + 1);
                state.last_p = p;
                state.last_t = t;
                continue;
            }
            if !state.lap_valid { state.last_p = p; state.last_t = t; continue; }
            let mut si = state.sector_idx;
            while si < n {
                let threshold = st.sector_starts[si];
                if !(last_p < threshold && p >= threshold) { break; }
                let t_cross = interpolate_time(last_p, last_t, p, t, threshold);
                let dur = (t_cross - state.sector_started_at) as f32;
                if dur > 0.0 { state.current_lap_sectors.push(dur); }
                state.sector_started_at = t_cross;
                state.sector_idx = si + 1;
                si += 1;
            }
            state.last_p = p;
            state.last_t = t;
        }
    }

    #[test]
    fn three_sector_lap() {
        let mut st = tracker(vec![1.0 / 3.0, 2.0 / 3.0]);

        let frames: Vec<(f64, f32, bool, i32)> = vec![
            (0.0,  0.95, false, 3),
            (1.0,  0.01, false, 3),
            (20.0, 0.32, false, 3),
            (21.0, 0.34, false, 3),
            (41.0, 0.65, false, 3),
            (42.0, 0.67, false, 3),
            (59.0, 0.97, false, 3),
            (60.0, 0.02, false, 3),
        ];

        feed(&mut st, &frames);

        let out = st.get(0).unwrap();
        assert_eq!(out.last_sectors.len(), 3, "should have 3 sector times");
        let s1 = out.last_sectors[0];
        assert!((s1 - 20.5).abs() < 1.0, "S1={s1}");
        for (i, &last) in out.last_sectors.iter().enumerate() {
            assert_eq!(out.personal_best[i], Some(last), "personal_best[{i}] should match last");
        }
    }

    #[test]
    fn personal_best_updates_on_improvement() {
        let mut st = tracker(vec![0.5]);

        let lap1: Vec<(f64, f32, bool, i32)> = vec![
            (0.0,  0.95, false, 3),
            (1.0,  0.01, false, 3),
            (21.0, 0.51, false, 3),
            (61.0, 0.98, false, 3),
            (62.0, 0.02, false, 3),
        ];
        feed(&mut st, &lap1);
        let pb_after_lap1 = st.get(0).unwrap().personal_best.clone();

        let lap2: Vec<(f64, f32, bool, i32)> = vec![
            (80.0, 0.51, false, 3),
            (125.0, 0.97, false, 3),
            (127.0, 0.02, false, 3),
        ];
        feed(&mut st, &lap2);
        let pb_after_lap2 = st.get(0).unwrap().personal_best.clone();

        assert!(
            pb_after_lap2[0] <= pb_after_lap1[0],
            "S1 best should improve or stay: {:?} vs {:?}", pb_after_lap2[0], pb_after_lap1[0]
        );
    }

    #[test]
    fn pit_invalidates_lap() {
        let mut st = tracker(vec![0.5]);

        let frames: Vec<(f64, f32, bool, i32)> = vec![
            (0.0,  0.95, false, 3),
            (1.0,  0.01, false, 3),
            (10.0, 0.45, true,  1),
            (20.0, 0.55, false, 3),
            (60.0, 0.98, false, 3),
            (61.0, 0.02, false, 3),
        ];
        feed(&mut st, &frames);

        let out = st.get(0).unwrap();
        assert!(out.last_sectors.is_empty(), "pit should invalidate lap");
    }

    #[test]
    fn no_sectors_is_noop() {
        let st = tracker(vec![]);
        assert!(st.get(0).is_none());
        assert!(st.session_best_sectors().is_empty());
    }

    #[test]
    fn drain_emits_completion_per_lap() {
        let mut st = tracker(vec![0.5]);

        // Two clean laps
        let frames: Vec<(f64, f32, bool, i32)> = vec![
            // Arm
            (0.0,  0.95, false, 3),
            // S/F crossing #1 → lap 1 starts
            (1.0,  0.01, false, 3),
            // Mid-S1
            (25.0, 0.51, false, 3),
            // S/F crossing #2 → lap 1 completes, lap 2 starts
            (62.0, 0.98, false, 3),
            (63.0, 0.02, false, 3),
            // Mid-S1 lap 2
            (85.0, 0.51, false, 3),
            // S/F crossing #3 → lap 2 completes
            (124.0, 0.97, false, 3),
            (125.0, 0.03, false, 3),
        ];
        feed(&mut st, &frames);

        let completions = st.drain_completed_laps();
        assert_eq!(completions.len(), 2, "should have 2 completions");
        assert_eq!(completions[0].lap_num, 1);
        assert_eq!(completions[1].lap_num, 2);
        assert!(completions[0].valid, "lap 1 should be valid");
        assert!(completions[1].valid, "lap 2 should be valid");
        assert!(!completions[0].in_lap, "lap 1 not an in-lap");

        // Drain again — should be empty
        assert!(st.drain_completed_laps().is_empty());
    }

    #[test]
    fn in_lap_flagged() {
        let mut st = tracker(vec![0.5]);

        let frames: Vec<(f64, f32, bool, i32)> = vec![
            (0.0,  0.95, false, 3),
            // S/F: lap 1 starts
            (1.0,  0.01, false, 3),
            // Pit entry during lap 1
            (20.0, 0.45, true, 1),
            (25.0, 0.50, false, 3), // back on track
            // S/F: lap 1 (pit-in) completes, lap 2 starts
            (61.0, 0.98, false, 3),
            (62.0, 0.02, false, 3),
        ];
        feed(&mut st, &frames);

        let completions = st.drain_completed_laps();
        assert_eq!(completions.len(), 1);
        assert!(completions[0].in_lap, "should be flagged as in-lap");
        assert!(!completions[0].valid, "pit lap should be invalid");
    }
}
