//! Per-car push-to-pass tracking, derived purely from the P2P timer, updated
//! every 4-Hz standings tick.
//!
//! iRacing's `CarIdxP2P_Status` flag is unreliable in live sessions (it stays
//! `false` even while a driver is clearly using P2P), so we ignore it entirely
//! and work only from the remaining-seconds timer (`CarIdxP2P_Count`, decoded
//! as `f32::from_bits(raw) * 10`; the player's own slot uses the plain integer
//! scalar `P2P_Count`):
//!   * the timer counting down  -> P2P is active,
//!   * the timer no longer counting down -> activity ended, cooldown starts
//!     (e.g. the SF23's mandatory 100s delay before P2P is usable again).
//!
//! The timer also carries two sentinel values that are *not* real budgets and
//! must be excluded from the decrease/cooldown logic:
//!   * `~999` -> P2P is unlimited (e.g. Practice), reported as [`P2pAvailability::Unlimited`],
//!   * `0`    -> P2P is unavailable / has no system (e.g. Qualifying), reported as
//!     [`P2pAvailability::Unavailable`].
//!
//! Treating the `999 -> 0` jump on a Practice -> Qualifying change as a real
//! decrease would otherwise start a phantom cooldown, so both sentinels reset
//! the per-car decrease state instead.
//!
//! The telemetry occasionally drops values for a tick or two, so active/cooldown
//! state is derived from the *last observed decrease* with grace/staleness
//! windows rather than from instantaneous transitions.

use crate::error::Result;
use crate::iracing_sdk::IRacingClient;
use serde::Serialize;
use std::collections::HashMap;
use ts_rs::TS;

/// Mandatory delay (seconds) after deactivating P2P before it becomes usable
/// again. Verified for the SF23.
const COOLDOWN_SECS: f64 = 100.0;
/// At/above this value the P2P count is the "unlimited" sentinel (~999, seen in
/// Practice), not a real remaining budget. Real budgets are far below this.
const UNLIMITED_THRESHOLD: f32 = 900.0;
/// At/below this value the P2P count means P2P is unavailable (no system, or
/// disabled e.g. in Qualifying).
const UNAVAILABLE_THRESHOLD: f32 = 0.05;
/// Minimum drop between two observations to count as the timer "decreasing"
/// (guards against float-decode noise).
const DECREASE_EPSILON: f32 = 0.01;
/// How long after the last observed decrease P2P is still considered active.
/// Bridges short data gaps and the ~1s steps of the player's integer counter
/// before we flip to cooldown.
const ACTIVE_GRACE_SECS: f64 = 2.0;
/// How long a held `remaining` value keeps being reported once data stops
/// arriving (smooths transient gaps; afterwards we report `None`).
const STALE_SECS: f64 = 3.0;

/// Whether a car's P2P is a real countdown, unlimited, or unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub enum P2pAvailability {
    /// A real remaining-seconds budget (raced sessions).
    Limited,
    /// ~999 sentinel: P2P is effectively infinite (e.g. Practice).
    Unlimited,
    /// 0 sentinel: P2P has no system / is disabled (e.g. Qualifying).
    #[default]
    Unavailable,
}

/// Classifies a raw P2P count value into an availability state.
fn classify(current: f32) -> P2pAvailability {
    if current <= UNAVAILABLE_THRESHOLD {
        P2pAvailability::Unavailable
    } else if current >= UNLIMITED_THRESHOLD {
        P2pAvailability::Unlimited
    } else {
        P2pAvailability::Limited
    }
}

#[derive(Debug, Default)]
struct CarState {
    /// Last valid timer value seen, only set while [`Self::availability`] is
    /// `Limited` (sentinels clear it to avoid phantom decreases).
    last_remaining: Option<f32>,
    /// SessionTime of the last observation (used for staleness of both the
    /// remaining value and the availability state).
    last_remaining_time: f64,
    /// SessionTime of the last real decrease in the timer.
    last_decrease_time: Option<f64>,
    /// Availability classified from the most recent observation.
    availability: P2pAvailability,
}

/// Derives per-car P2P remaining / active / cooldown from the timer alone.
#[derive(Debug, Default)]
pub struct P2pTracker {
    cars: HashMap<i32, CarState>,
    last_sub_session_id: i64,
    session_time: f64,
}

impl P2pTracker {
    /// Read the current P2P timer for all cars and advance tracking.
    /// `sub_session_id` triggers a full reset when the sub-session changes so
    /// that state from a previous session does not bleed into the next one.
    pub fn update(&mut self, client: &IRacingClient, sub_session_id: i64) -> Result<()> {
        if sub_session_id != self.last_sub_session_id {
            self.cars.clear();
            self.last_sub_session_id = sub_session_id;
        }
        self.session_time = client.get_f64("SessionTime")?;

        // CarIdxP2P_Count is declared Int but carries raw Float32 bits
        // (× 10 = seconds) for opponents; the player's own slot instead mirrors
        // the plain-integer scalar `P2P_Count`. See project_p2p_encoding memory.
        let Ok(counts) = client.get_i32_array("CarIdxP2P_Count") else {
            return Ok(());
        };
        let player_car_idx = client.get_i32("PlayerCarIdx").ok();
        let player_p2p_count = client.get_i32("P2P_Count").ok();

        let t = self.session_time;
        for (idx, &raw) in counts.iter().enumerate() {
            let car_idx = idx as i32;
            let current = if player_car_idx == Some(car_idx) {
                player_p2p_count.filter(|&c| c >= 0).map(|c| c as f32)
            } else {
                Some(f32::from_bits(raw as u32) * 10.0).filter(|s| s.is_finite() && *s >= 0.0)
            };
            // A gap (no valid value) leaves the state untouched so it doesn't
            // falsely trigger a cooldown.
            let Some(current) = current else { continue };

            let state = self.cars.entry(car_idx).or_default();
            state.availability = classify(current);
            match state.availability {
                // Sentinels (~999 unlimited, 0 unavailable) are not real
                // budgets: drop the remaining value and any in-flight cooldown
                // so e.g. a Practice -> Qualifying `999 -> 0` jump does not
                // register as a decrease and start a phantom cooldown.
                P2pAvailability::Unlimited | P2pAvailability::Unavailable => {
                    state.last_remaining = None;
                    state.last_decrease_time = None;
                }
                P2pAvailability::Limited => {
                    if let Some(prev) = state.last_remaining {
                        if current + DECREASE_EPSILON < prev {
                            state.last_decrease_time = Some(t);
                        }
                    }
                    state.last_remaining = Some(current);
                }
            }
            state.last_remaining_time = t;
        }
        Ok(())
    }

    /// Last known remaining P2P seconds, held briefly across data gaps. `None`
    /// once the value goes stale or the car has no P2P system.
    pub fn remaining(&self, car_idx: i32) -> Option<f32> {
        let s = self.cars.get(&car_idx)?;
        (self.session_time - s.last_remaining_time < STALE_SECS)
            .then_some(s.last_remaining)
            .flatten()
    }

    /// Availability of the car's P2P (`Limited`/`Unlimited`/`Unavailable`),
    /// held within the staleness window. Defaults to `Unavailable` for unknown
    /// or stale cars (a car we cannot observe has no usable P2P).
    pub fn availability(&self, car_idx: i32) -> P2pAvailability {
        self.cars
            .get(&car_idx)
            .filter(|s| self.session_time - s.last_remaining_time < STALE_SECS)
            .map(|s| s.availability)
            .unwrap_or_default()
    }

    /// True while the timer has decreased within the active-grace window.
    pub fn is_active(&self, car_idx: i32) -> bool {
        self.cars
            .get(&car_idx)
            .and_then(|s| s.last_decrease_time)
            .is_some_and(|d| self.session_time - d < ACTIVE_GRACE_SECS)
    }

    /// Seconds remaining in the post-deactivation cooldown, measured from the
    /// last observed decrease (≈ release moment), or `None` while still active
    /// or once the cooldown has expired.
    pub fn cooldown_remaining(&self, car_idx: i32) -> Option<f32> {
        let d = self.cars.get(&car_idx)?.last_decrease_time?;
        let since = self.session_time - d;
        if since < ACTIVE_GRACE_SECS {
            return None; // still active
        }
        let remaining = COOLDOWN_SECS - since;
        (remaining > 0.0).then_some(remaining as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Record an observation directly (mirrors the per-car logic in `update`).
    fn observe(tracker: &mut P2pTracker, car_idx: i32, current: f32, t: f64) {
        tracker.session_time = t;
        let state = tracker.cars.entry(car_idx).or_default();
        state.availability = classify(current);
        match state.availability {
            P2pAvailability::Unlimited | P2pAvailability::Unavailable => {
                state.last_remaining = None;
                state.last_decrease_time = None;
            }
            P2pAvailability::Limited => {
                if let Some(prev) = state.last_remaining {
                    if current + DECREASE_EPSILON < prev {
                        state.last_decrease_time = Some(t);
                    }
                }
                state.last_remaining = Some(current);
            }
        }
        state.last_remaining_time = t;
    }

    #[test]
    fn decreasing_timer_is_active() {
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 50.0, 100.0);
        observe(&mut tracker, 0, 49.5, 100.25);
        assert!(tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), None);
    }

    #[test]
    fn first_observation_is_not_active() {
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 50.0, 100.0);
        assert!(!tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), None);
    }

    #[test]
    fn stall_starts_cooldown_after_grace() {
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 50.0, 100.0);
        observe(&mut tracker, 0, 49.5, 100.25); // last decrease at 100.25

        // Within grace: still active, no cooldown.
        tracker.session_time = 101.0;
        assert!(tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), None);

        // Past grace: cooldown counts from the last decrease.
        tracker.session_time = 110.25; // since = 10.0
        assert!(!tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), Some(90.0));
    }

    #[test]
    fn cooldown_expires() {
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 50.0, 100.0);
        observe(&mut tracker, 0, 49.5, 100.0); // decrease at t=100.0

        tracker.session_time = 199.9;
        assert!(tracker.cooldown_remaining(0).unwrap() > 0.0);

        tracker.session_time = 200.0;
        assert_eq!(tracker.cooldown_remaining(0), None);
    }

    #[test]
    fn short_gap_does_not_trigger_cooldown() {
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 50.0, 100.0);
        observe(&mut tracker, 0, 49.5, 100.25);

        // No update for ~1s (data gap), still within grace.
        tracker.session_time = 101.25;
        assert!(tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), None);

        // Data returns, still decreasing -> stays active.
        observe(&mut tracker, 0, 49.0, 101.5);
        assert!(tracker.is_active(0));
    }

    #[test]
    fn remaining_held_then_goes_stale() {
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 50.0, 100.0);

        // Within staleness window: held.
        tracker.session_time = 102.0;
        assert_eq!(tracker.remaining(0), Some(50.0));

        // Past staleness window: dropped.
        tracker.session_time = 104.0;
        assert_eq!(tracker.remaining(0), None);
    }

    #[test]
    fn unknown_car_has_no_state() {
        let tracker = P2pTracker::default();
        assert_eq!(tracker.remaining(0), None);
        assert!(!tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), None);
        assert_eq!(tracker.availability(0), P2pAvailability::Unavailable);
    }

    #[test]
    fn zero_is_unavailable() {
        assert_eq!(classify(0.0), P2pAvailability::Unavailable);
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 0.0, 100.0);
        assert_eq!(tracker.availability(0), P2pAvailability::Unavailable);
        assert_eq!(tracker.remaining(0), None);
    }

    #[test]
    fn high_value_is_unlimited() {
        assert_eq!(classify(999.0), P2pAvailability::Unlimited);
        assert_eq!(classify(999.9), P2pAvailability::Unlimited);
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 999.9, 100.0);
        assert_eq!(tracker.availability(0), P2pAvailability::Unlimited);
        // Unlimited is a sentinel, not a real remaining value.
        assert_eq!(tracker.remaining(0), None);
    }

    #[test]
    fn unlimited_then_unavailable_does_not_start_cooldown() {
        // Practice (unlimited) -> Qualifying (unavailable): the 999 -> 0 drop
        // must NOT be read as a release that starts the cooldown.
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 999.0, 100.0);
        observe(&mut tracker, 0, 0.0, 100.25);
        assert_eq!(tracker.availability(0), P2pAvailability::Unavailable);
        assert!(!tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), None);
    }

    #[test]
    fn unlimited_then_limited_no_phantom_decrease() {
        // Practice (unlimited) -> Race (a real budget): the 999 -> 50 drop must
        // not register as a decrease (no active/cooldown from the transition).
        let mut tracker = P2pTracker::default();
        observe(&mut tracker, 0, 999.0, 100.0);
        observe(&mut tracker, 0, 50.0, 100.25);
        assert_eq!(tracker.availability(0), P2pAvailability::Limited);
        assert!(!tracker.is_active(0));
        assert_eq!(tracker.cooldown_remaining(0), None);
        assert_eq!(tracker.remaining(0), Some(50.0));
    }
}
