//! Pit-related rules.

use std::time::Duration;

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::EngineerState;

/// High frequency only: every single inc, normal updates. Subject to pit-lane mute.
pub struct IncidentHighRule;
/// Medium+ frequency: warning when ≤4 points remaining before the limit.
/// Declared Critical so it bypasses the 3s global cooldown.
pub struct IncidentMediumRule;
/// All frequencies: critical alert when ≤2 points remaining. Bypasses everything.
pub struct IncidentLowRule;

/// Suggest pitting when fuel window opens (~5 laps of fuel left, first stint).
pub struct ConsiderPitRule;
/// Drive-through penalty detected via increased incident count.
pub struct DrivethroughPenaltyRule;
/// Briefing on exit from pit lane.
pub struct PitlaneExitBriefingRule;

impl Rule for ConsiderPitRule {
    fn id(&self) -> &'static str { "consider_pit" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(120) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let fl = current.fuel_laps_left;
        if fl <= 0.0 || fl < 1.5 {
            return None; // FuelCritical handles the critical end
        }
        if current.in_pit || current.in_garage {
            return None;
        }
        // Suppress in the opening laps
        if current.total_laps_driven < 3 {
            return None;
        }
        let prev_fl = prev.map(|p| p.fuel_laps_left).unwrap_or(f32::MAX);
        if fl < 5.0 && prev_fl >= 5.0 {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "consider_pit",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for DrivethroughPenaltyRule {
    fn id(&self) -> &'static str { "drivethrough_penalty" }
    fn priority(&self) -> Priority { Priority::Critical }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        // Fire on rising edge of the actual iRacing drive-through penalty flag
        // (CarIdxSessionFlags bit 0x10000000). Incident-count changes are NOT penalties.
        if current.has_drivethrough_penalty && !prev.has_drivethrough_penalty {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "drivethrough_penalty",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for PitlaneExitBriefingRule {
    fn id(&self) -> &'static str { "pit_exit_briefing" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::PRACTICE | SessionMask::QUALIFYING | SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        // Fire when exiting pit lane
        if !prev.in_pit || current.in_pit {
            return None;
        }

        let track_condition = if current.rain_intensity >= 0.3 {
            "wet"
        } else if current.rain_intensity >= 0.05 {
            "damp"
        } else {
            "dry"
        };

        let avg_temp = (current.tire_temps_c[0]
            + current.tire_temps_c[1]
            + current.tire_temps_c[2]
            + current.tire_temps_c[3]) / 4.0;

        let tyre_status = if avg_temp > 0.1 && avg_temp < 60.0 {
            "cold"
        } else if (60.0..80.0).contains(&avg_temp) {
            "warming"
        } else {
            "ready"
        };

        Some(RuleEvent {
            rule_id: self.id(),
            priority: self.priority(),
            template_key: "pit_exit_briefing",
            params: TemplateParams::new()
                .set("track_condition", track_condition.to_string())
                .set("tyre_status", tyre_status.to_string())
                .set("temp", format!("{:.0}", current.ambient_temp_c))
                .set("track_temp", format!("{:.0}", current.track_temp_c))
                .set("air_temp", format!("{:.0}", current.ambient_temp_c)),
        })
    }
}

// ---------------------------------------------------------------------------
// Incident rules — three levels matching the three frequency settings
// ---------------------------------------------------------------------------
//
// Frequency logic:
//   Low    → only IncidentLowRule fires  (critical: ≤2 remaining before kick)
//   Medium → IncidentLowRule + IncidentMediumRule (warning: ≤4 remaining)
//   High   → all three (every single inc is announced)
//
// When no incident limit is set:
//   Low/Medium → silent (no limit = no stakes)
//   High       → every inc reported ("X points on the board")
//
// IncidentMediumRule and IncidentLowRule declare Priority::Critical at the
// rule level so they bypass the 3s global non-critical cooldown and are never
// delayed by other callouts. IncidentHighRule (informational updates) is
// Priority::High and is subject to the global cooldown and pit-lane mute.

impl Rule for IncidentHighRule {
    fn id(&self) -> &'static str { "incident_high" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::ZERO }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        if current.incident_count <= prev.incident_count {
            return None;
        }
        let count = current.incident_count;
        let delta = current.incident_count - prev.incident_count;
        let mut params = TemplateParams::new()
            .set("count", count.to_string())
            .set("delta", delta.to_string());

        let template_key = match current.incident_limit {
            None => "incident_update_no_limit",
            Some(limit) => {
                let remaining = limit.saturating_sub(count);
                // Let Medium/Low rules handle the warning and critical zones
                if remaining <= 4 {
                    return None;
                }
                params = params
                    .set("limit", limit.to_string())
                    .set("remaining", remaining.to_string());
                "incident_update"
            }
        };

        Some(RuleEvent {
            rule_id: self.id(),
            priority: self.priority(),
            template_key,
            params,
        })
    }
}

impl Rule for IncidentMediumRule {
    fn id(&self) -> &'static str { "incident_medium" }
    // Declared Critical so this isn't delayed by the 3s global non-critical cooldown.
    // The emitted event carries Priority::High (shown in amber in the transcript).
    fn priority(&self) -> Priority { Priority::Critical }
    fn cooldown(&self) -> Duration { Duration::ZERO }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        if current.incident_count <= prev.incident_count {
            return None;
        }
        let count = current.incident_count;
        let limit = current.incident_limit?; // no limit = no warning threshold
        let remaining = limit.saturating_sub(count);
        // Only handle the warning zone (>2 remaining handled here, ≤2 goes to Low rule)
        if remaining > 4 || remaining <= 2 {
            return None;
        }
        let params = TemplateParams::new()
            .set("count", count.to_string())
            .set("limit", limit.to_string())
            .set("remaining", remaining.to_string());
        Some(RuleEvent {
            rule_id: self.id(),
            priority: Priority::High,
            template_key: "incident_warning",
            params,
        })
    }
}

impl Rule for IncidentLowRule {
    fn id(&self) -> &'static str { "incident_low" }
    // Critical: bypasses global cooldown, pit-lane mute, and garage mute doesn't
    // apply (handled in dispatcher). Always gets through.
    fn priority(&self) -> Priority { Priority::Critical }
    fn cooldown(&self) -> Duration { Duration::ZERO }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&mut self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        if current.incident_count <= prev.incident_count {
            return None;
        }
        let count = current.incident_count;
        let limit = current.incident_limit?; // no limit = no critical threshold
        let remaining = limit.saturating_sub(count);
        if remaining > 2 {
            return None;
        }
        let params = TemplateParams::new()
            .set("count", count.to_string())
            .set("limit", limit.to_string())
            .set("remaining", remaining.to_string());
        Some(RuleEvent {
            rule_id: self.id(),
            priority: Priority::Critical,
            template_key: "incident_critical",
            params,
        })
    }
}
