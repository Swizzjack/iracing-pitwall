//! Fuel rules — Critical and Low warnings.

use std::time::Duration;

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::EngineerState;

/// Critical: fuel_laps_left < 1.5 → pit immediately.
pub struct FuelCriticalRule;
/// Low: fuel_laps_left crosses below 3.5 on the way down.
pub struct FuelLowRule;

impl Rule for FuelCriticalRule {
    fn id(&self) -> &'static str { "fuel_critical" }
    fn priority(&self) -> Priority { Priority::Critical }
    fn cooldown(&self) -> Duration { Duration::from_secs(45) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let fl = current.fuel_laps_left;
        if fl <= 0.0 {
            return None; // no valid fuel data
        }
        if current.total_laps_driven < 1 {
            return None; // don't fire before we've done a lap
        }
        // Only fire on downward crossing
        let prev_fl = prev.map(|p| p.fuel_laps_left).unwrap_or(f32::MAX);
        if fl < 1.5 && prev_fl >= 1.5 {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "fuel_critical_box",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for FuelLowRule {
    fn id(&self) -> &'static str { "fuel_low" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let fl = current.fuel_laps_left;
        if fl <= 0.0 || current.total_laps_driven < 1 {
            return None;
        }
        // Don't overlap with critical rule
        if fl < 1.5 {
            return None;
        }
        let prev_fl = prev.map(|p| p.fuel_laps_left).unwrap_or(f32::MAX);
        if fl < 3.5 && prev_fl >= 3.5 {
            let laps = format!("{:.0}", fl);
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "fuel_low",
                params: TemplateParams::new().set("laps", laps),
            })
        } else {
            None
        }
    }
}
