//! Per-car push-to-pass cooldown tracking, updated every 4-Hz standings tick.
//!
//! Some cars (e.g. SF23) enforce a mandatory delay after P2P is deactivated
//! before it can be used again. We watch `CarIdxP2P_Status` for active→inactive
//! transitions and derive the remaining cooldown from `SessionTime`.

use crate::error::Result;
use crate::iracing_sdk::IRacingClient;
use std::collections::HashMap;

/// Mandatory delay (seconds) after deactivating P2P before it becomes usable
/// again. Verified for the SF23.
const COOLDOWN_SECS: f64 = 100.0;

#[derive(Debug, Default)]
struct CarState {
    was_active: bool,
    deactivated_at: Option<f64>,
}

/// Tracks P2P active→inactive transitions to derive a per-car cooldown countdown.
#[derive(Debug, Default)]
pub struct P2pTracker {
    cars: HashMap<i32, CarState>,
    last_sub_session_id: i64,
    session_time: f64,
}

impl P2pTracker {
    /// Read current P2P status for all cars and advance cooldown tracking.
    /// `sub_session_id` triggers a full reset when the sub-session changes so
    /// that cooldowns from a previous session do not bleed into the next one.
    pub fn update(&mut self, client: &IRacingClient, sub_session_id: i64) -> Result<()> {
        if sub_session_id != self.last_sub_session_id {
            self.cars.clear();
            self.last_sub_session_id = sub_session_id;
        }
        self.session_time = client.get_f64("SessionTime")?;
        if let Ok(status) = client.get_bool_array("CarIdxP2P_Status") {
            for (idx, &now_active) in status.iter().enumerate() {
                let state = self.cars.entry(idx as i32).or_default();
                advance(state, now_active, self.session_time);
            }
        }
        Ok(())
    }

    /// Seconds remaining until P2P becomes usable again, or `None` if the car
    /// isn't currently within its post-deactivation cooldown window.
    pub fn cooldown_remaining(&self, car_idx: i32) -> Option<f32> {
        let deactivated_at = self.cars.get(&car_idx)?.deactivated_at?;
        let remaining = COOLDOWN_SECS - (self.session_time - deactivated_at);
        (remaining > 0.0).then_some(remaining as f32)
    }
}

fn advance(state: &mut CarState, now_active: bool, t: f64) {
    if state.was_active && !now_active {
        state.deactivated_at = Some(t);
    }
    state.was_active = now_active;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deactivation_starts_cooldown() {
        let mut s = CarState::default();
        advance(&mut s, true, 100.0);
        assert_eq!(s.deactivated_at, None);

        advance(&mut s, false, 112.5);
        assert_eq!(s.deactivated_at, Some(112.5));
    }

    #[test]
    fn cooldown_counts_down_and_expires() {
        let mut tracker = P2pTracker::default();
        let state = tracker.cars.entry(0).or_default();
        advance(state, true, 100.0);
        advance(state, false, 100.0);

        tracker.session_time = 150.0;
        assert_eq!(tracker.cooldown_remaining(0), Some(50.0));

        tracker.session_time = 199.9;
        assert!(tracker.cooldown_remaining(0).unwrap() > 0.0);

        tracker.session_time = 200.0;
        assert_eq!(tracker.cooldown_remaining(0), None);
    }

    #[test]
    fn reactivation_clears_cooldown() {
        let mut tracker = P2pTracker::default();
        let state = tracker.cars.entry(0).or_default();
        advance(state, true, 100.0);
        advance(state, false, 100.0);
        tracker.session_time = 120.0;
        assert!(tracker.cooldown_remaining(0).is_some());

        let state = tracker.cars.get_mut(&0).unwrap();
        advance(state, true, 130.0);
        assert_eq!(state.deactivated_at, Some(100.0));
        advance(state, false, 140.0);
        assert_eq!(state.deactivated_at, Some(140.0));
    }

    #[test]
    fn no_status_no_cooldown() {
        let tracker = P2pTracker::default();
        assert_eq!(tracker.cooldown_remaining(0), None);
    }
}
