//! Flag rules — Red, Yellow, Blue, Green, Debris.

use std::time::Duration;

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::{EngineerState, SessionPhase};

pub struct RedFlagRule;
pub struct YellowFlagOwnSectorRule;
pub struct BlueFlagRule;
pub struct GreenFlagRule;
pub struct DebrisFlagRule;
pub struct MeatballFlagRule;

impl Rule for RedFlagRule {
    fn id(&self) -> &'static str { "red_flag" }
    fn priority(&self) -> Priority { Priority::Critical }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        // Red flag is stored in FlagState.red (derived from session_flags bit 0x10)
        // Also fire on SessionPhase::RedFlag (same source, belt-and-suspenders)
        let is_red = current.active_flags.red
            || current.session_phase == SessionPhase::RedFlag;
        let was_red = prev.map(|p| {
            p.active_flags.red || p.session_phase == SessionPhase::RedFlag
        }).unwrap_or(false);

        if is_red && !was_red {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "red_flag",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for YellowFlagOwnSectorRule {
    fn id(&self) -> &'static str { "yellow_flag_sector" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(20) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let now_under = current.active_flags.player_in_yellow_sector;
        let prev_under = prev.map(|p| p.active_flags.player_in_yellow_sector).unwrap_or(false);
        if now_under && !prev_under {
            let sector = current.active_flags.player_sector_idx + 1;
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "yellow_flag_sector",
                params: TemplateParams::new().set("sector", sector.to_string()),
            })
        } else {
            None
        }
    }
}

impl Rule for BlueFlagRule {
    fn id(&self) -> &'static str { "blue_flag" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(20) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let now_blue = current.active_flags.blue;
        let prev_blue = prev.map(|p| p.active_flags.blue).unwrap_or(false);
        if now_blue && !prev_blue {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "blue_flag",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for DebrisFlagRule {
    fn id(&self) -> &'static str { "debris_flag" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(10) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let now_debris = current.active_flags.debris;
        let was_debris = prev.map(|p| p.active_flags.debris).unwrap_or(false);
        if now_debris && !was_debris {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "debris_flag",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for MeatballFlagRule {
    fn id(&self) -> &'static str { "meatball_flag" }
    fn priority(&self) -> Priority { Priority::Critical }
    fn cooldown(&self) -> Duration { Duration::from_secs(60) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let now_meatball = current.active_flags.meatball;
        let was_meatball = prev.map(|p| p.active_flags.meatball).unwrap_or(false);
        if now_meatball && !was_meatball {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "meatball_flag",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for GreenFlagRule {
    fn id(&self) -> &'static str { "green_flag" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(15) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        let prev_had_yellow =
            prev.active_flags.player_in_yellow_sector || prev.active_flags.red;
        let now_clear = !current.active_flags.player_in_yellow_sector
            && !current.active_flags.red;
        if prev_had_yellow && now_clear {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "green_flag",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}
