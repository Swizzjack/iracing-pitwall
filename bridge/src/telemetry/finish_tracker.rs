//! Race finish detection for the standings header badge.
//!
//! Latches the checkered flag (cars still finishing) and the CoolDown state
//! (final official result) so `StandingsSnapshot` can surface the
//! `Live → Finishing → Final` lifecycle. Reset logic: whenever SubSessionID or
//! SessionNum changes all state is cleared so a new subsession / session starts
//! fresh.

use crate::error::Result;
use crate::iracing_sdk::IRacingClient;

/// iRacing SessionFlags bit for the checkered flag (irsdk_checkered = 0x1).
/// 0x4 is the GREEN flag — do not confuse the two.
const IRSDK_CHECKERED: u32 = 0x0000_0001;

/// iRacing `SessionState` value for CoolDown — the session is over and
/// `ResultsPositions` holds the final official classification. Enum:
/// 0=Invalid 1=GetInCar 2=Warmup 3=ParadeLaps 4=Racing 5=Checkered 6=CoolDown
/// (see `race_engineer::state::SessionPhase`). NOTE: 5 (Checkered) means the
/// leader finished but the race may still be running — only 6 is truly final.
const SESSION_STATE_COOLDOWN: i32 = 6;

/// Tracks the checkered-flag and CoolDown lifecycle for the standings badge.
#[derive(Debug, Default)]
pub struct FinishTracker {
    last_subsession_id: i64,
    last_session_num: i32,
    checkered_seen: bool,
    /// Latched once `SessionState` reaches CoolDown (6) — the session is over
    /// and `ResultsPositions` is the final official classification.
    finished_seen: bool,
    /// Set to true once `checkered_edge_fired()` has returned `Some`.
    edge_consumed: bool,
}

impl FinishTracker {
    /// Call once per standings tick before `StandingsSnapshot::build`.
    /// `sub_session_id` is the *effective* (possibly synthetic) ID.
    /// Reads `SessionFlags` and `SessionState` to update internal state.
    pub fn observe(
        &mut self,
        client: &IRacingClient,
        sub_session_id: i64,
        session_num: i32,
    ) -> Result<()> {
        // Reset on session/subsession change.
        if sub_session_id != self.last_subsession_id || session_num != self.last_session_num {
            self.last_subsession_id = sub_session_id;
            self.last_session_num = session_num;
            self.checkered_seen = false;
            self.finished_seen = false;
            self.edge_consumed = false;
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

        Ok(())
    }

    /// Whether the session-wide checkered flag has been observed.
    pub fn checkered(&self) -> bool {
        self.checkered_seen
    }

    /// Whether the session has reached CoolDown — i.e. the race is fully over
    /// and `ResultsPositions` is the final official result.
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
}
