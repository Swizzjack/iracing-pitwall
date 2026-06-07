//! Session timing rules — 5 minutes remaining, last lap, race finished.

use std::time::Duration;

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::{EngineerState, SessionPhase, SessionType};

pub struct FiveMinutesRemainingRule;
pub struct LastLapRule;
pub struct RaceFinishedRule;

impl Rule for FiveMinutesRemainingRule {
    fn id(&self) -> &'static str { "five_minutes_remaining" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(3600) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE | SessionMask::QUALIFYING }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let remaining = current.time_remaining?.as_secs_f64();
        let prev_remaining = prev.and_then(|p| p.time_remaining).map(|d| d.as_secs_f64());

        let now_five = remaining <= 300.0;
        let was_five = prev_remaining.map(|r| r <= 300.0).unwrap_or(false);

        if now_five && !was_five {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "five_minutes",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for LastLapRule {
    fn id(&self) -> &'static str { "last_lap" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(3600) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE | SessionMask::QUALIFYING }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        match current.session_type {
            SessionType::Race => {
                let laps = current.laps_remaining?;
                let prev_laps = prev.and_then(|p| p.laps_remaining);
                if laps == 1 && prev_laps.map(|l| l > 1).unwrap_or(true) {
                    Some(RuleEvent {
                        rule_id: self.id(),
                        priority: self.priority(),
                        template_key: "last_lap",
                        params: TemplateParams::new(),
                    })
                } else {
                    None
                }
            }
            SessionType::Qualifying => {
                // Fire when ~90 s remain so the driver can start one final flying lap
                let remaining = current.time_remaining?.as_secs_f64();
                let prev_remaining = prev
                    .and_then(|p| p.time_remaining)
                    .map(|d| d.as_secs_f64());
                let now_last = remaining <= 90.0;
                let was_last = prev_remaining.map(|r| r <= 90.0).unwrap_or(false);
                if now_last && !was_last {
                    Some(RuleEvent {
                        rule_id: self.id(),
                        priority: self.priority(),
                        template_key: "last_lap",
                        params: TemplateParams::new(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Rule for RaceFinishedRule {
    fn id(&self) -> &'static str { "race_finished" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(3600) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        let now_done = matches!(
            current.session_phase,
            SessionPhase::Finished | SessionPhase::Checkered
        );
        let was_done = matches!(
            prev.session_phase,
            SessionPhase::Finished | SessionPhase::Checkered
        );
        if now_done && !was_done {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "race_finished",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}
