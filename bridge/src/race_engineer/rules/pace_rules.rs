//! Pace rules — personal best, pace dropping, sector deltas, class rivals.

use std::time::Duration;

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::EngineerState;

pub struct PersonalBestRule;
pub struct PaceDroppingRule;
pub struct SectorDeltaRule;
pub struct SessionBestOvertakenRule;
pub struct ClassAheadSlowerRule;
pub struct ClassAheadFasterRule;
pub struct ClassBehindFasterRule;
pub struct ClassBehindSlowerRule;
pub struct ClassBestLapRule;
/// Summary of class-best pace and player delta — fires in all sessions.
pub struct ClassPaceBriefRule;
/// Announces the current session-best lap in class periodically — chatty only.
pub struct SessionBestPaceRule;

/// Format a lap time Duration as "m:ss.xx" (e.g. "1:23.45").
fn fmt_lap(d: Duration) -> String {
    let total = d.as_secs_f64();
    let minutes = (total / 60.0) as u64;
    let seconds = total - (minutes as f64 * 60.0);
    format!("{minutes}:{seconds:05.2}")
}

/// Returns true if a new lap has just been completed (lap counter incremented).
fn lap_just_completed(current: &EngineerState, prev: Option<&EngineerState>) -> bool {
    prev.map(|p| current.player_lap > p.player_lap).unwrap_or(false)
}

impl Rule for PersonalBestRule {
    fn id(&self) -> &'static str { "personal_best" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !current.personal_best_this_lap {
            return None;
        }
        let mut params = TemplateParams::new();
        if let Some(t) = current.last_lap_time {
            params = params.set("lap_time", fmt_lap(t));
        }
        Some(RuleEvent {
            rule_id: self.id(),
            priority: self.priority(),
            template_key: "personal_best",
            params,
        })
    }
}

impl Rule for PaceDroppingRule {
    fn id(&self) -> &'static str { "pace_dropping" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) {
            return None;
        }
        let laps = &current.recent_lap_times;
        if laps.len() < 4 {
            return None;
        }

        let pb = current.best_lap_time_personal?.as_secs_f32();
        let cutoff = pb * 1.10; // ignore laps >10% slower than PB (outliers)

        let valid: Vec<f32> = laps
            .iter()
            .map(|d| d.as_secs_f32())
            .filter(|&t| t < cutoff)
            .collect();

        if valid.len() < 4 {
            return None;
        }

        let recent_avg: f32 = valid[valid.len().saturating_sub(3)..].iter().sum::<f32>()
            / 3.0_f32.min(valid.len() as f32);
        let base_avg: f32 = valid[..valid.len().saturating_sub(3)].iter().sum::<f32>()
            / (valid.len().saturating_sub(3)) as f32;

        // Pace is dropping if recent avg is >1.5 s slower than earlier laps
        if recent_avg > base_avg + 1.5 {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "pace_dropping",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for SectorDeltaRule {
    fn id(&self) -> &'static str { "sector_delta" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::QUALIFYING }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        prev?;
        // Find the first sector with a significant delta (>0.3 s)
        for (i, &delta) in current.last_sector_deltas.iter().enumerate() {
            if let Some(d) = delta {
                if d.abs() > 0.3 {
                    return Some(RuleEvent {
                        rule_id: self.id(),
                        priority: self.priority(),
                        template_key: "sector_delta",
                        params: TemplateParams::new()
                            .set("sector", (i + 1).to_string())
                            .set("delta", format!("{:+.2}", d)),
                    });
                }
            }
        }
        None
    }
}

impl Rule for SessionBestOvertakenRule {
    fn id(&self) -> &'static str { "session_best_overtaken" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(60) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE | SessionMask::QUALIFYING }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) {
            return None;
        }
        let prev = prev?;
        let cur_session_best = current.best_lap_time_session?;
        let prev_session_best = prev.best_lap_time_session.unwrap_or(Duration::MAX);
        let cur_pb = current.best_lap_time_personal?;

        // Session best improved AND the player is not the one who set it
        if cur_session_best < prev_session_best && cur_session_best != cur_pb {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "session_best_overtaken",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for ClassBestLapRule {
    fn id(&self) -> &'static str { "class_best_lap" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE | SessionMask::QUALIFYING }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) {
            return None;
        }
        let pb = current.best_lap_time_personal?;
        let session_best = current.best_lap_time_session?;
        if pb == session_best {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "class_best_lap",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

fn class_pace_event(
    rule_id: &'static str,
    template_key: &'static str,
    rival: Option<Duration>,
    player: Option<Duration>,
    faster_is_trigger: bool, // true = rival faster, false = rival slower
) -> Option<RuleEvent> {
    let rival_s = rival?.as_secs_f32();
    let player_s = player?.as_secs_f32();
    let delta = rival_s - player_s; // negative = rival faster
    let threshold = 0.5; // 0.5 s/lap difference to trigger

    let triggered = if faster_is_trigger {
        delta < -threshold // rival faster
    } else {
        delta > threshold  // rival slower
    };

    if triggered {
        let gap = format!("{:.1}", delta.abs());
        Some(RuleEvent {
            rule_id,
            priority: Priority::Info,
            template_key,
            params: TemplateParams::new().set("gap", gap),
        })
    } else {
        None
    }
}

impl Rule for ClassAheadSlowerRule {
    fn id(&self) -> &'static str { "class_ahead_slower" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) { return None; }
        class_pace_event(
            self.id(),
            "class_ahead_slower",
            current.class_car_ahead_last_lap,
            current.last_lap_time,
            false,
        )
    }
}

impl Rule for ClassAheadFasterRule {
    fn id(&self) -> &'static str { "class_ahead_faster" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) { return None; }
        class_pace_event(
            self.id(),
            "class_ahead_faster",
            current.class_car_ahead_last_lap,
            current.last_lap_time,
            true,
        )
    }
}

impl Rule for ClassBehindFasterRule {
    fn id(&self) -> &'static str { "class_behind_faster" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) { return None; }
        class_pace_event(
            self.id(),
            "class_behind_faster",
            current.class_car_behind_last_lap,
            current.last_lap_time,
            true,
        )
    }
}

impl Rule for ClassBehindSlowerRule {
    fn id(&self) -> &'static str { "class_behind_slower" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) { return None; }
        class_pace_event(
            self.id(),
            "class_behind_slower",
            current.class_car_behind_last_lap,
            current.last_lap_time,
            false,
        )
    }
}

impl Rule for ClassPaceBriefRule {
    fn id(&self) -> &'static str { "class_pace_brief" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(90) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) {
            return None;
        }
        let rival_best = current.class_rivals_min_best_lap?;
        let player_best = current.best_lap_time_personal?;

        let rival_s = rival_best.as_secs_f64();
        let player_s = player_best.as_secs_f64();
        let delta = player_s - rival_s; // positive = player is slower

        let mut params = TemplateParams::new()
            .set("class_best", fmt_lap(rival_best))
            .set("delta", format!("{:+.2}", delta));

        if let Some(avg) = current.class_rivals_avg_last_lap {
            params = params.set("field_avg", fmt_lap(avg));
        }

        Some(RuleEvent {
            rule_id: self.id(),
            priority: self.priority(),
            template_key: "class_pace_brief",
            params,
        })
    }
}

impl Rule for SessionBestPaceRule {
    fn id(&self) -> &'static str { "session_best_pace" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if !lap_just_completed(current, prev) {
            return None;
        }
        let session_best = current.best_lap_time_session?;
        Some(RuleEvent {
            rule_id: self.id(),
            priority: self.priority(),
            template_key: "session_best_pace",
            params: TemplateParams::new().set("class_best", fmt_lap(session_best)),
        })
    }
}
