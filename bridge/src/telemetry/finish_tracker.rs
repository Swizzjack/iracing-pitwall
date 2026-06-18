//! Per-car finish detection: freeze a car's StandingEntry the tick it
//! crosses the start/finish line under the checkered flag.
//!
//! Reset logic: whenever SubSessionID or SessionNum changes all state is
//! cleared so a new subsession / session starts fresh.

use crate::error::Result;
use crate::iracing_sdk::IRacingClient;
use crate::telemetry::standings::StandingEntry;
use std::collections::{HashMap, HashSet};

/// iRacing SessionFlags bit for the checkered flag (irsdk_checkered = 0x1).
/// 0x4 is the GREEN flag — do not confuse the two.
const IRSDK_CHECKERED: u32 = 0x0000_0001;

/// iRacing `SessionState` value for CoolDown — the session is over and
/// `ResultsPositions` holds the final official classification. Enum:
/// 0=Invalid 1=GetInCar 2=Warmup 3=ParadeLaps 4=Racing 5=Checkered 6=CoolDown
/// (see `race_engineer::state::SessionPhase`). NOTE: 5 (Checkered) means the
/// leader finished but the race may still be running — only 6 is truly final.
const SESSION_STATE_COOLDOWN: i32 = 6;

/// Tracks which cars have crossed the S/F line under the checkered flag and
/// stores frozen copies of their StandingEntry from that moment.
#[derive(Debug, Default)]
pub struct FinishTracker {
    last_subsession_id: i64,
    last_session_num: i32,
    /// False until the first `observe()` call after a reset.
    initialized: bool,
    checkered_seen: bool,
    /// Latched once `SessionState` reaches CoolDown (6) — the session is over
    /// and `ResultsPositions` is the final official classification.
    finished_seen: bool,
    /// Set to true once `checkered_edge_fired()` has returned `Some`.
    edge_consumed: bool,
    /// CarIdxLap value from the previous observe() call.
    prev_lap: HashMap<i32, i32>,
    /// Cars whose CarIdxLap incremented in the most recent observe() call.
    incremented_this_tick: HashSet<i32>,
    /// Frozen entries — set once, never overwritten.
    frozen: HashMap<i32, StandingEntry>,
}

impl FinishTracker {
    /// Call once per standings tick before `StandingsSnapshot::build`.
    /// `sub_session_id` is the *effective* (possibly synthetic) ID.
    /// Reads `SessionFlags` and `CarIdxLap` to update internal state.
    pub fn observe(
        &mut self,
        client: &IRacingClient,
        sub_session_id: i64,
        session_num: i32,
    ) -> Result<()> {
        // Reset on session/subsession change.
        if sub_session_id != self.last_subsession_id
            || session_num != self.last_session_num
        {
            self.last_subsession_id = sub_session_id;
            self.last_session_num = session_num;
            self.initialized = false;
            self.checkered_seen = false;
            self.finished_seen = false;
            self.edge_consumed = false;
            self.prev_lap.clear();
            self.incremented_this_tick.clear();
            self.frozen.clear();
            log::info!(
                "finish_tracker: reset for subsession={} session={}",
                sub_session_id,
                session_num
            );
        }

        // Latch checkered flag — stays true for the rest of the session.
        let flags = client.get_bitfield("SessionFlags")?;
        if flags & IRSDK_CHECKERED != 0 {
            if !self.checkered_seen {
                log::info!("finish_tracker: checkered flag seen");
            }
            self.checkered_seen = true;
        }

        // Latch the truly-final state. Unlike the checkered flag (leader done,
        // race may continue), CoolDown means the session has ended and the
        // ResultsPositions YAML carries the final official classification.
        if client.get_i32("SessionState")? == SESSION_STATE_COOLDOWN {
            if !self.finished_seen {
                log::info!("finish_tracker: session entered CoolDown (final results)");
            }
            self.finished_seen = true;
        }

        // Detect lap-counter increments vs. previous tick.
        self.incremented_this_tick.clear();
        let laps = client.get_i32_array("CarIdxLap")?;
        if self.initialized {
            for (idx, &lap) in laps.iter().enumerate() {
                let car_idx = idx as i32;
                let prev = self.prev_lap.get(&car_idx).copied().unwrap_or(-1);
                // lap >= 0 guards against the -1 "not on track" sentinel.
                if lap > prev && lap >= 0 {
                    self.incremented_this_tick.insert(car_idx);
                }
            }
        }
        // Advance prev_lap (skip -1 entries — keep the last valid value).
        for (idx, &lap) in laps.iter().enumerate() {
            if lap >= 0 {
                self.prev_lap.insert(idx as i32, lap);
            }
        }
        self.initialized = true;
        Ok(())
    }

    /// Whether the session-wide checkered flag has been observed.
    pub fn checkered(&self) -> bool {
        self.checkered_seen
    }

    /// Whether the session has reached CoolDown — i.e. the race is fully over
    /// and `ResultsPositions` is the final official result. Use this (not
    /// `checkered()`) to gate the results-based standings overwrite.
    pub fn session_finished(&self) -> bool {
        self.finished_seen
    }

    /// Returns the current `sub_session_id` exactly once when the checkered flag
    /// transitions from unseen to seen. Returns `None` on all subsequent calls
    /// (and after a reset). Use this to trigger a one-shot results fetch.
    pub fn checkered_edge_fired(&mut self) -> Option<i64> {
        if self.checkered_seen && !self.edge_consumed {
            self.edge_consumed = true;
            Some(self.last_subsession_id)
        } else {
            None
        }
    }

    /// Whether this car's lap counter incremented in the current tick.
    pub fn has_incremented(&self, car_idx: i32) -> bool {
        self.incremented_this_tick.contains(&car_idx)
    }

    /// Freeze `entry` for `car_idx` if no frozen entry already exists.
    pub fn freeze_if_new(&mut self, car_idx: i32, entry: StandingEntry) {
        self.frozen.entry(car_idx).or_insert(entry);
    }

    /// Return the frozen entry for `car_idx`, if any.
    pub fn frozen_entry(&self, car_idx: i32) -> Option<&StandingEntry> {
        self.frozen.get(&car_idx)
    }
}
