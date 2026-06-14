//! RuleDispatcher — evaluates all rules at 10 Hz.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{FrequencyLevel, Priority, Rule, RuleEvent, SessionMask};
use crate::race_engineer::state::{EngineerState, SessionType};

// ---------------------------------------------------------------------------
// EngineerBehavior — user-controlled settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EngineerBehavior {
    pub enabled: bool,
    pub frequency: FrequencyLevel,
    pub mute_in_qualifying: bool,
    /// When true and session = Practice: all rules run regardless of SessionMask
    pub debug_all_rules_in_practice: bool,
    /// Voice ID to synthesize with. None = no synthesis.
    pub active_voice: Option<String>,
    /// Pilot name injected into `{driver_name}` placeholders. None / "" = omit.
    pub pilot_name: Option<String>,
    /// When true the name is never injected even if `pilot_name` is set.
    pub mute_name: bool,
}

impl Default for EngineerBehavior {
    fn default() -> Self {
        Self {
            enabled: false,
            frequency: FrequencyLevel::Medium,
            mute_in_qualifying: false,
            debug_all_rules_in_practice: false,
            active_voice: None,
            pilot_name: None,
            mute_name: false,
        }
    }
}

// ---------------------------------------------------------------------------
// RuleDispatcher
// ---------------------------------------------------------------------------

/// Minimum gap between two non-Critical callouts (3 seconds).
const GLOBAL_COOLDOWN: Duration = Duration::from_secs(3);

pub struct RuleDispatcher {
    rules: Vec<Box<dyn Rule>>,
    /// rule_id → last fired `Instant` (per-rule cooldown)
    cooldowns: HashMap<&'static str, Instant>,
    /// Last time any non-Critical event was dispatched
    last_non_critical_fire: Option<Instant>,
    pub behavior: EngineerBehavior,
    /// Rolling counter for template variant selection (cheap pseudo-random)
    render_counter: usize,
}

impl RuleDispatcher {
    pub fn new(rules: Vec<Box<dyn Rule>>, behavior: EngineerBehavior) -> Self {
        Self {
            rules,
            cooldowns: HashMap::new(),
            last_non_critical_fire: None,
            behavior,
            render_counter: 0,
        }
    }

    /// Process one 10 Hz tick. Returns events sorted Critical → High → Info.
    pub fn tick(
        &mut self,
        current: &EngineerState,
        previous: Option<&EngineerState>,
    ) -> Vec<RuleEvent> {
        if !self.behavior.enabled {
            return Vec::new();
        }
        if self.behavior.mute_in_qualifying
            && current.session_type == SessionType::Qualifying
        {
            return Vec::new();
        }

        let effective_session_mask = if self.behavior.debug_all_rules_in_practice
            && current.session_type == SessionType::Practice
        {
            SessionMask::ALL
        } else {
            current.session_type.to_mask()
        };

        let freq_mask = self.behavior.frequency.to_mask();
        let now = Instant::now();
        let mut events: Option<Vec<RuleEvent>> = None;

        for rule in &mut self.rules {
            // Session filter
            if !rule.session_mask().intersects(effective_session_mask) {
                continue;
            }
            // Frequency filter
            if !rule.frequency_mask().contains(freq_mask) {
                continue;
            }
            // Garage: mute everything
            if current.in_garage {
                continue;
            }
            // Pit lane: mute non-Critical
            if current.in_pit && rule.priority() != Priority::Critical {
                continue;
            }
            // Per-rule cooldown
            if let Some(&last_fired) = self.cooldowns.get(rule.id()) {
                if now.duration_since(last_fired) < rule.cooldown() {
                    continue;
                }
            }
            // Global cooldown (non-Critical only)
            if rule.priority() != Priority::Critical {
                if let Some(last_nc) = self.last_non_critical_fire {
                    if now.duration_since(last_nc) < GLOBAL_COOLDOWN {
                        continue;
                    }
                }
            }

            if let Some(event) = rule.evaluate(current, previous) {
                log::info!(
                    "Rule fired: {} priority={} template={}",
                    event.rule_id,
                    event.priority.as_str(),
                    event.template_key,
                );
                self.cooldowns.insert(rule.id(), now);
                if event.priority != Priority::Critical {
                    self.last_non_critical_fire = Some(now);
                }
                events.get_or_insert_with(Vec::new).push(event);
            }
        }

        let mut events = events.unwrap_or_default();
        // Critical first, then High, then Info; stable within tier (FIFO)
        events.sort_by_key(|e| std::cmp::Reverse(e.priority));
        events
    }

    pub fn next_render_seed(&mut self) -> usize {
        self.render_counter = self.render_counter.wrapping_add(1);
        self.render_counter
    }

    pub fn update_behavior(&mut self, behavior: EngineerBehavior) {
        self.behavior = behavior;
    }
}
