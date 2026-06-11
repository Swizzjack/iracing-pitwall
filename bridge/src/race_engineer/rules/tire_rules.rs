//! Tire temperature and wear rules.

use std::time::Duration;

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::EngineerState;

const TEMP_HOT: f32 = 105.0;
const TEMP_COLD: f32 = 70.0;

pub struct TireTempsOutOfRangeRule;
pub struct TireTempsInRangeRule;
pub struct TireWear50Rule;
pub struct TireWear75Rule;
pub struct TireWear90Rule;

fn max_temp(state: &EngineerState) -> f32 {
    state.tire_temps_c.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
}

fn min_temp(state: &EngineerState) -> f32 {
    state.tire_temps_c.iter().cloned().fold(f32::INFINITY, f32::min)
}

fn max_wear(state: &EngineerState) -> f32 {
    state.tire_wear_pct.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
}

fn any_out_of_range(state: &EngineerState) -> bool {
    // Only check if tyres have data (non-zero)
    let max_t = max_temp(state);
    let min_t = min_temp(state);
    max_t > 0.1 && (max_t > TEMP_HOT || min_t < TEMP_COLD)
}

fn all_in_range(state: &EngineerState) -> bool {
    let max_t = max_temp(state);
    let min_t = min_temp(state);
    max_t > 0.1 && max_t <= TEMP_HOT && min_t >= TEMP_COLD
}

impl Rule for TireTempsOutOfRangeRule {
    fn id(&self) -> &'static str { "tire_temps_out_of_range" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(90) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let now_out = any_out_of_range(current);
        let was_out = prev.map(any_out_of_range).unwrap_or(false);
        if now_out && !was_out {
            let key = if max_temp(current) > TEMP_HOT {
                "tire_temps_hot"
            } else {
                "tire_temps_cold"
            };
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: key,
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for TireTempsInRangeRule {
    fn id(&self) -> &'static str { "tire_temps_in_range" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        let now_ok = all_in_range(current);
        let was_out = any_out_of_range(prev);
        if now_ok && was_out {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "tire_temps_cold",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for TireWear50Rule {
    fn id(&self) -> &'static str { "tire_wear_50" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(300) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let wear = max_wear(current);
        let prev_wear = prev.map(max_wear).unwrap_or(0.0);
        if wear >= 0.50 && prev_wear < 0.50 {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "tire_wear_50",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for TireWear75Rule {
    fn id(&self) -> &'static str { "tire_wear_75" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let wear = max_wear(current);
        let prev_wear = prev.map(max_wear).unwrap_or(0.0);
        if wear >= 0.75 && prev_wear < 0.75 {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "tire_wear_75",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for TireWear90Rule {
    fn id(&self) -> &'static str { "tire_wear_90" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(60) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let wear = max_wear(current);
        let prev_wear = prev.map(max_wear).unwrap_or(0.0);
        if wear >= 0.90 && prev_wear < 0.90 {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "tire_wear_90",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}
