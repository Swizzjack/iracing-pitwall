//! Smooths the transient "+1L" flicker in race standings.
//!
//! iRacing's `ResultsPositions` only refreshes a car's `LapsComplete`/`Time`
//! when that car crosses the start/finish line. The moment the leader starts a
//! new lap, every car still on the current lap has a `LapsComplete` one lower
//! than the leader's, so a naive gap calc reports them all as "+1L" — even cars
//! only a few seconds behind — until they cross the line themselves and a fresh
//! time gap is published.
//!
//! This tracker keeps the last published gap for a car until that car's own
//! `LapsComplete` advances (i.e. it has crossed S/F, so a genuinely new value is
//! available). A lap-down is only committed once the car's own crossing confirms
//! it; the leader lapping the field no longer flips everyone to "+1L" mid-lap.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
struct GapState {
    /// Last gap value emitted for this car (seconds, or a negative integer
    /// encoding laps down).
    gap: f32,
    /// The car's own `laps_complete` when `gap` was last computed from fresh
    /// data — a change here means the car has crossed S/F again.
    car_laps: i32,
}

/// Per-car gap smoothing across standings ticks. Reset on session change.
#[derive(Debug, Default)]
pub struct GapTracker {
    last_session_num: i32,
    state: HashMap<i32, GapState>,
}

impl GapTracker {
    /// Clears all state when the active session changes (P → Q → R) so gaps
    /// never leak across sessions.
    pub fn reset_if_session_changed(&mut self, session_num: i32) {
        if session_num != self.last_session_num {
            self.last_session_num = session_num;
            self.state.clear();
        }
    }

    /// Returns the gap to display for `car_idx`.
    ///
    /// * `car_laps` — the car's own `LapsComplete` (advances only when it
    ///   crosses S/F → signals that a fresh value is available).
    /// * `lap_delta` — `leader.laps_complete - car_laps` (`<= 0` ⇒ on the lead
    ///   lap, a real time gap is available; `>= 1` ⇒ currently shown a lap down).
    /// * `time_gap` — `r.time - leader.time` in seconds, valid on the lead lap.
    pub fn resolve(&mut self, car_idx: i32, car_laps: i32, lap_delta: i32, time_gap: f32) -> f32 {
        if lap_delta <= 0 {
            // Fresh, authoritative time gap — publish and remember it.
            self.state.insert(car_idx, GapState { gap: time_gap, car_laps });
            return time_gap;
        }
        // Shown a lap behind the leader. While the car has not crossed S/F since
        // we last recorded a value (the leader merely pulled ahead onto a new
        // lap), keep the previous gap. Only commit the lap-down once the car's
        // own LapsComplete has advanced, confirming it really is a lap down.
        match self.state.get(&car_idx) {
            Some(prev) if prev.car_laps == car_laps => prev.gap,
            _ => {
                let gap = -(lap_delta as f32);
                self.state.insert(car_idx, GapState { gap, car_laps });
                gap
            }
        }
    }
}
