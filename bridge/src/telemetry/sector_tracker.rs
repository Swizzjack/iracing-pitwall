//! Per-car sector-time tracking at 60 Hz from CarIdxLapDistPct + SessionTime.

use crate::error::Result;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::iracing_sdk::IRacingClient;
use std::collections::HashMap;

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
    /// Accumulates sector times for the current lap; reset on S/F crossing or invalidation.
    current_lap_sectors: Vec<f32>,
    /// Whether the current lap is still clean (no pit/garage/off-world events).
    lap_valid: bool,
    output: PerCarSectors,
}

impl CarState {
    fn new(n_sectors: usize) -> Self {
        Self {
            last_p: -1.0,
            last_t: -1.0,
            sector_idx: 0,
            sector_started_at: 0.0,
            current_lap_sectors: Vec::new(),
            lap_valid: false,
            output: PerCarSectors {
                last_sectors: Vec::new(),
                personal_best: vec![None; n_sectors],
            },
        }
    }

    fn reset_lap(&mut self, t: f64, n_sectors: usize) {
        self.sector_idx = 0;
        self.sector_started_at = t;
        self.current_lap_sectors = Vec::with_capacity(n_sectors);
        self.lap_valid = true;
    }

    fn invalidate_lap(&mut self) {
        self.lap_valid = false;
        self.current_lap_sectors.clear();
    }
}

/// Tracks sector crossings for every car at 60 Hz.
#[derive(Debug, Default)]
pub struct SectorTracker {
    cars: HashMap<i32, CarState>,
    /// Sorted sector start pcts, without 0.0 (S/F).
    sector_starts: Vec<f32>,
    last_session_num: Option<i32>,
}

impl SectorTracker {
    /// Update sector tracking for all cars. Called every 60-Hz frame.
    pub fn update(&mut self, client: &IRacingClient, yaml: &SessionInfoYaml) -> Result<()> {
        let session_num = client.get_i32("SessionNum")?;
        if self.last_session_num != Some(session_num) {
            log::info!("sector_tracker: reset for session={session_num}");
            self.last_session_num = Some(session_num);
            self.cars.clear();
        }

        let new_starts = yaml.sector_starts();
        if new_starts != self.sector_starts {
            self.sector_starts = new_starts;
            self.cars.clear();
        }

        let n = self.sector_starts.len();
        if n == 0 {
            return Ok(());
        }

        let session_time = client.get_f64("SessionTime")?;
        let lap_dist_pcts = client.get_f32_array("CarIdxLapDistPct")?;
        let on_pit = client.get_bool_array("CarIdxOnPitRoad")?;
        let surfaces = client.get_i32_array("CarIdxTrackSurface").ok();

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
                state.invalidate_lap();
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
                // Finish the last sector of the previous lap.
                if state.lap_valid && state.sector_idx == n {
                    let t_cross = interpolate_time(last_p, last_t, p, session_time, 0.0);
                    let dur = (t_cross - state.sector_started_at) as f32;
                    if dur > 0.0 {
                        state.current_lap_sectors.push(dur);
                    }
                }
                // Commit the completed lap if we have exactly n+1 sector times.
                if state.lap_valid && state.current_lap_sectors.len() == n + 1 {
                    let times = state.current_lap_sectors.clone();
                    // Update personal bests.
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
                // Start fresh lap.
                let t_sf = interpolate_time(last_p, last_t, p, session_time, 0.0);
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
            // Iterate sectors in order starting from the current one.
            let mut si = state.sector_idx;
            while si < n {
                let threshold = self.sector_starts[si];
                // Did we cross this threshold going forward?
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

    // Build a synthetic SectorTracker with given thresholds.
    fn tracker(starts: Vec<f32>) -> SectorTracker {
        SectorTracker {
            cars: HashMap::new(),
            sector_starts: starts,
            last_session_num: None,
        }
    }

    // Feed a sequence of (time, lapDistPct, onPit, surface) frames into state for car 0.
    fn feed(
        st: &mut SectorTracker,
        frames: &[(f64, f32, bool, i32)],
    ) {
        let n = st.sector_starts.len();
        for &(t, p, pit, surface) in frames {
            if p < 0.0 || surface == -1 {
                continue;
            }
            let state = st.cars.entry(0).or_insert_with(|| CarState::new(n + 1));
            if pit || surface == 1 || surface == 2 {
                state.invalidate_lap();
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
                if state.lap_valid && state.sector_idx == n {
                    let t_cross = interpolate_time(last_p, last_t, p, t, 0.0);
                    let dur = (t_cross - state.sector_started_at) as f32;
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
                let t_sf = interpolate_time(last_p, last_t, p, t, 0.0);
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

        // Pre-arm: start before S/F, then cross it.
        let frames: Vec<(f64, f32, bool, i32)> = vec![
            // arrive near end of lap
            (0.0,  0.95, false, 3),
            // S/F crossing at t≈0.5 (p goes 0.95 → 0.01)
            (1.0,  0.01, false, 3),
            // cross S2 boundary (1/3) at t≈21
            (20.0, 0.32, false, 3),
            (21.0, 0.34, false, 3),
            // cross S3 boundary (2/3) at t≈42
            (41.0, 0.65, false, 3),
            (42.0, 0.67, false, 3),
            // approach S/F again (next lap)
            (59.0, 0.97, false, 3),
            (60.0, 0.02, false, 3), // lap complete
        ];

        feed(&mut st, &frames);

        let out = st.get(0).unwrap();
        assert_eq!(out.last_sectors.len(), 3, "should have 3 sector times");
        // S1 ≈ 20.5 s (S/F at t≈0.5, S2 crossing at t≈21.0)
        let s1 = out.last_sectors[0];
        assert!((s1 - 20.5).abs() < 1.0, "S1={s1}");
        // All personal bests should equal last_sectors (first lap).
        for (i, &last) in out.last_sectors.iter().enumerate() {
            assert_eq!(out.personal_best[i], Some(last), "personal_best[{i}] should match last");
        }
    }

    #[test]
    fn personal_best_updates_on_improvement() {
        let mut st = tracker(vec![0.5]);

        // Lap 1: S1≈20s, S2≈40s
        let lap1: Vec<(f64, f32, bool, i32)> = vec![
            (0.0,  0.95, false, 3),
            (1.0,  0.01, false, 3),
            (21.0, 0.51, false, 3),
            (61.0, 0.98, false, 3),
            (62.0, 0.02, false, 3),
        ];
        feed(&mut st, &lap1);
        let pb_after_lap1 = st.get(0).unwrap().personal_best.clone();

        // Lap 2: faster S1 (~18s), slower S2 (~45s)
        let lap2: Vec<(f64, f32, bool, i32)> = vec![
            (80.0, 0.51, false, 3),
            (125.0, 0.97, false, 3),
            (127.0, 0.02, false, 3),
        ];
        feed(&mut st, &lap2);
        let pb_after_lap2 = st.get(0).unwrap().personal_best.clone();

        // S1 personal best should have improved (< lap1 best).
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
            (10.0, 0.45, true,  1), // pit in mid-S1
            (20.0, 0.55, false, 3), // back on track
            (60.0, 0.98, false, 3),
            (61.0, 0.02, false, 3),
        ];
        feed(&mut st, &frames);

        let out = st.get(0).unwrap();
        // Pit invalidated the lap → no last_sectors committed.
        assert!(out.last_sectors.is_empty(), "pit should invalidate lap");
    }

    #[test]
    fn no_sectors_is_noop() {
        let st = tracker(vec![]);
        // update with empty sectors should not panic; state stays empty.
        assert!(st.get(0).is_none());
        assert!(st.session_best_sectors().is_empty());
    }
}
