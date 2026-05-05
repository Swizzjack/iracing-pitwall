//! Per-car pit-road timing state, updated every 4-Hz standings tick.

use crate::error::Result;
use crate::iracing_sdk::IRacingClient;
use std::collections::HashMap;

/// Pit-timing data exported per car for inclusion in StandingEntry.
#[derive(Debug, Clone, Default)]
pub struct PitInfo {
    pub pit_stops: u32,
    pub last_pit_road_sec: Option<f32>,
    pub current_pit_road_sec: Option<f32>,
}

#[derive(Debug, Default)]
struct CarState {
    on_pit_road: bool,
    entered_at: f64,
    info: PitInfo,
}

/// Tracks pit-road entry/exit transitions across standings ticks.
#[derive(Debug, Default)]
pub struct PitTracker {
    cars: HashMap<i32, CarState>,
}

impl PitTracker {
    /// Read current iRacing state and advance pit timing for all cars.
    pub fn update(&mut self, client: &IRacingClient) -> Result<()> {
        let session_time = client.get_f64("SessionTime")?;
        let on_pit = client.get_bool_array("CarIdxOnPitRoad")?;
        for (idx, &now_on) in on_pit.iter().enumerate() {
            let state = self.cars.entry(idx as i32).or_default();
            advance(state, now_on, session_time);
        }
        Ok(())
    }

    pub fn get(&self, car_idx: i32) -> Option<&PitInfo> {
        self.cars.get(&car_idx).map(|s| &s.info)
    }
}

fn advance(state: &mut CarState, now_on: bool, t: f64) {
    match (state.on_pit_road, now_on) {
        (false, true) => {
            state.on_pit_road = true;
            state.entered_at = t;
            state.info.current_pit_road_sec = Some(0.0);
        }
        (true, false) => {
            let dur = ((t - state.entered_at) as f32).max(0.0);
            state.info.last_pit_road_sec = Some(dur);
            state.info.pit_stops += 1;
            state.info.current_pit_road_sec = None;
            state.on_pit_road = false;
        }
        (true, true) => {
            state.info.current_pit_road_sec = Some(((t - state.entered_at) as f32).max(0.0));
        }
        (false, false) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_then_exit_counts_stop() {
        let mut s = CarState::default();
        advance(&mut s, true, 100.0);
        assert!(s.on_pit_road);
        assert_eq!(s.info.current_pit_road_sec, Some(0.0));
        assert_eq!(s.info.pit_stops, 0);

        advance(&mut s, true, 112.5);
        assert_eq!(s.info.current_pit_road_sec, Some(12.5));

        advance(&mut s, false, 125.5);
        assert!(!s.on_pit_road);
        assert_eq!(s.info.pit_stops, 1);
        assert_eq!(s.info.last_pit_road_sec, Some(25.5));
        assert_eq!(s.info.current_pit_road_sec, None);
    }

    #[test]
    fn second_stop_increments_counter() {
        let mut s = CarState::default();
        advance(&mut s, true, 200.0);
        advance(&mut s, false, 230.0);
        advance(&mut s, true, 500.0);
        advance(&mut s, false, 535.0);
        assert_eq!(s.info.pit_stops, 2);
        assert_eq!(s.info.last_pit_road_sec, Some(35.0));
    }

    #[test]
    fn no_entry_no_change() {
        let mut s = CarState::default();
        advance(&mut s, false, 50.0);
        assert_eq!(s.info.pit_stops, 0);
        assert_eq!(s.info.last_pit_road_sec, None);
        assert_eq!(s.info.current_pit_road_sec, None);
    }
}
