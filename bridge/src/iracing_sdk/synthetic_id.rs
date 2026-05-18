//! Synthetic SubSessionID for offline / AI sessions that report sub_session_id = 0.
//!
//! iRacing assigns real IDs only for server-hosted sessions. In offline and AI modes
//! the MMF always contains sub_session_id = 0, so every run would collapse into the
//! same DB row. This module generates a stable negative ID per offline run that
//! changes when the "run signature" (track + session-type layout) changes.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::iracing_sdk::types::{SessionEntry, WeekendInfo};

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Global monotonic counter — ensures each new offline run gets a unique ID
/// even if two runs start within the same millisecond (e.g. in tests).
static OFFLINE_SEQ: AtomicI64 = AtomicI64::new(0);

#[derive(Debug, Default)]
pub struct SyntheticSubSessionId {
    /// The synthetic id currently in use (negative), or None if using a real id.
    current: Option<i64>,
    /// Signature that was used when `current` was generated.
    last_sig: Option<(i64, i32, String)>,
}

impl SyntheticSubSessionId {
    /// Resolve the effective sub_session_id.
    ///
    /// - If `raw_sub_id > 0`: pass through, clear synthetic state.
    /// - If `raw_sub_id == 0`: return a stable negative ID for this offline run.
    ///   A new ID is generated when the run signature changes (different track or
    ///   session-type layout), so each offline reload gets its own DB entry.
    pub fn resolve(
        &mut self,
        raw_sub_id: i64,
        weekend: &WeekendInfo,
        sessions: &[SessionEntry],
    ) -> i64 {
        if raw_sub_id > 0 {
            self.current = None;
            self.last_sig = None;
            return raw_sub_id;
        }

        let sig = build_signature(weekend, sessions);
        if let (Some(id), Some(ref last)) = (self.current, &self.last_sig) {
            if *last == sig {
                return id;
            }
        }
        // New offline run detected — generate a fresh negative ID.
        // Combine timestamp (ms) with a monotonic seq so concurrent/fast calls never collide.
        let seq = OFFLINE_SEQ.fetch_add(1, Ordering::Relaxed);
        let new_id = -(unix_now() * 1_000_000 + seq + 1);
        self.current = Some(new_id);
        self.last_sig = Some(sig);
        log::info!("synthetic_id: new offline sub_session_id={new_id} (track_id={})", weekend.track_id);
        new_id
    }

    /// Reset on SDK disconnect so the next connect starts fresh.
    pub fn reset(&mut self) {
        self.current = None;
        self.last_sig = None;
    }
}

fn build_signature(weekend: &WeekendInfo, sessions: &[SessionEntry]) -> (i64, i32, String) {
    // Sorted session-type names as a stable string.
    let mut types: Vec<&str> = sessions.iter().map(|s| s.session_type.as_str()).collect();
    types.sort_unstable();
    types.dedup();
    let layout = types.join(",");
    (weekend.track_id, weekend.series_id, layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iracing_sdk::types::{SessionEntry, WeekendInfo};

    fn weekend(track_id: i64, series_id: i32) -> WeekendInfo {
        WeekendInfo {
            track_name: String::new(),
            track_display_name: String::new(),
            track_id,
            track_config_name: String::new(),
            track_length: String::new(),
            series_id,
            session_id: 0,
            sub_session_id: 0,
            track_weather_type: None,
            track_city: None,
            track_country: None,
            track_altitude: None,
            track_num_turns: None,
            track_pit_speed_limit: None,
        }
    }

    fn session(t: &str) -> SessionEntry {
        SessionEntry {
            session_num: 0,
            session_type: t.to_string(),
            session_name: String::new(),
            session_track_rubber_state: None,
            results_positions: None,
        }
    }

    #[test]
    fn real_id_passes_through() {
        let mut s = SyntheticSubSessionId::default();
        let w = weekend(1, 1);
        let sess = vec![session("Race")];
        assert_eq!(s.resolve(12345, &w, &sess), 12345);
        assert!(s.current.is_none());
    }

    #[test]
    fn offline_id_is_negative() {
        let mut s = SyntheticSubSessionId::default();
        let w = weekend(1, 1);
        let sess = vec![session("Race")];
        let id = s.resolve(0, &w, &sess);
        assert!(id < 0);
    }

    #[test]
    fn same_signature_returns_same_id() {
        let mut s = SyntheticSubSessionId::default();
        let w = weekend(1, 1);
        let sess = vec![session("Practice"), session("Race")];
        let id1 = s.resolve(0, &w, &sess);
        let id2 = s.resolve(0, &w, &sess);
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_track_gets_new_id() {
        let mut s = SyntheticSubSessionId::default();
        let w1 = weekend(1, 1);
        let w2 = weekend(2, 1);
        let sess = vec![session("Race")];
        let id1 = s.resolve(0, &w1, &sess);
        let id2 = s.resolve(0, &w2, &sess);
        assert_ne!(id1, id2);
    }

    #[test]
    fn reset_forces_new_id() {
        let mut s = SyntheticSubSessionId::default();
        let w = weekend(1, 1);
        let sess = vec![session("Race")];
        let id1 = s.resolve(0, &w, &sess);
        s.reset();
        let id2 = s.resolve(0, &w, &sess);
        // After reset the signature is gone so a new id is always generated.
        assert_ne!(id1, id2);
    }
}
