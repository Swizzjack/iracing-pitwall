//! Rule engine for the race engineer.
//!
//! Each racing event is modeled as a `Rule` that is evaluated at 10 Hz.
//! Rules fire on threshold crossings and return a `RuleEvent` when triggered.

use std::collections::HashMap;
use std::time::Duration;

use bitflags::bitflags;

pub mod dispatcher;
pub mod templates;

// Rule category modules
pub mod damage_rules;
pub mod flag_rules;
pub mod fuel_rules;
pub mod pace_rules;
pub mod pit_rules;
pub mod position_rules;
pub mod session_rules;
pub mod tire_rules;
pub mod weather_rules;

use crate::race_engineer::state::EngineerState;

// ---------------------------------------------------------------------------
// Priority
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Info,
    High,
    Critical,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Info => "info",
            Priority::High => "high",
            Priority::Critical => "critical",
        }
    }
}

// ---------------------------------------------------------------------------
// Session / Frequency masks
// ---------------------------------------------------------------------------

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SessionMask: u8 {
        const PRACTICE   = 0b001;
        const QUALIFYING = 0b010;
        const RACE       = 0b100;
        const ALL        = 0b111;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FrequencyMask: u8 {
        const LOW            = 0b001;
        const MEDIUM         = 0b010;
        const HIGH           = 0b100;
        const MEDIUM_AND_UP  = 0b110;
        const ALL            = 0b111;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyLevel {
    Low,
    Medium,
    High,
}

impl FrequencyLevel {
    pub fn to_mask(self) -> FrequencyMask {
        match self {
            FrequencyLevel::Low => FrequencyMask::LOW,
            FrequencyLevel::Medium => FrequencyMask::MEDIUM,
            FrequencyLevel::High => FrequencyMask::HIGH,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "low" => FrequencyLevel::Low,
            "high" => FrequencyLevel::High,
            _ => FrequencyLevel::Medium,
        }
    }
}

// ---------------------------------------------------------------------------
// Template params
// ---------------------------------------------------------------------------

/// Lightweight key→value store for template placeholder substitution.
/// Allocated only when a rule fires.
#[derive(Debug, Clone, Default)]
pub struct TemplateParams(HashMap<String, String>);

impl TemplateParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, key: &str, value: String) -> Self {
        self.0.insert(key.to_string(), value);
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// Rule event
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct RuleEvent {
    pub rule_id: &'static str,
    pub priority: Priority,
    pub template_key: &'static str,
    pub params: TemplateParams,
}

// ---------------------------------------------------------------------------
// Rule trait
// ---------------------------------------------------------------------------

/// A single firing rule evaluated per 10 Hz tick.
///
/// `Send` because the dispatcher lives inside a Tokio task; rules are owned
/// exclusively by the dispatcher, so stateful rules keep plain fields and
/// take `&mut self` — no interior mutability needed.
pub trait Rule: Send {
    fn id(&self) -> &'static str;
    fn priority(&self) -> Priority;
    fn cooldown(&self) -> Duration;
    fn session_mask(&self) -> SessionMask;
    fn frequency_mask(&self) -> FrequencyMask;

    /// Called per tick when all gate conditions pass.
    /// Returns `Some(RuleEvent)` if the rule fires, `None` otherwise.
    fn evaluate(
        &mut self,
        current: &EngineerState,
        previous: Option<&EngineerState>,
    ) -> Option<RuleEvent>;
}

// ---------------------------------------------------------------------------
// Default rule set constructor
// ---------------------------------------------------------------------------

pub fn build_default_rules() -> Vec<Box<dyn Rule>> {
    use damage_rules::DamageReportedRule;
    use flag_rules::{BlueFlagRule, DebrisFlagRule, GreenFlagRule, MeatballFlagRule, RedFlagRule, YellowFlagOwnSectorRule};
    use fuel_rules::{FuelCriticalRule, FuelLowRule};
    use pace_rules::{
        ClassAheadFasterRule, ClassAheadSlowerRule, ClassBehindFasterRule, ClassBehindSlowerRule,
        ClassBestLapRule, ClassPaceBriefRule, PaceDroppingRule, PersonalBestRule, SectorDeltaRule,
        SessionBestOvertakenRule, SessionBestPaceRule,
    };
    use pit_rules::{ConsiderPitRule, DrivethroughPenaltyRule, IncidentHighRule, IncidentLowRule, IncidentMediumRule, PitlaneExitBriefingRule};
    use position_rules::{
        GapAheadHighRule, GapAheadMediumRule, GapBehindHighRule, GapBehindMediumRule,
        PositionGainedRule, PositionLostRule,
    };
    use session_rules::{FiveMinutesRemainingRule, LastLapRule, RaceFinishedRule};
    use tire_rules::{
        TireTempsInRangeRule, TireTempsOutOfRangeRule, TireWear50Rule, TireWear75Rule,
        TireWear90Rule,
    };
    use weather_rules::{
        AmbientTempChangeRule, RainClearingRule, RainEscalationRule, RainForecastWarningRule,
        RainStartingRule, TrackDryingRule, TrackTempChangeRule, WeatherBriefingRule,
    };

    vec![
        // Flags — always highest priority
        Box::new(RedFlagRule),
        Box::new(MeatballFlagRule),
        Box::new(YellowFlagOwnSectorRule),
        Box::new(BlueFlagRule),
        Box::new(GreenFlagRule),
        Box::new(DebrisFlagRule),

        // Fuel
        Box::new(FuelCriticalRule),
        Box::new(FuelLowRule),

        // Pit / Penalties
        Box::new(ConsiderPitRule),
        Box::new(DrivethroughPenaltyRule),
        Box::new(IncidentLowRule),
        Box::new(IncidentMediumRule),
        Box::new(IncidentHighRule),
        Box::new(PitlaneExitBriefingRule),

        // Damage
        Box::new(DamageReportedRule),

        // Session timing
        Box::new(FiveMinutesRemainingRule),
        Box::new(LastLapRule),
        Box::new(RaceFinishedRule),

        // Position / gaps
        Box::new(PositionGainedRule::default()),
        Box::new(PositionLostRule::default()),
        Box::new(GapAheadMediumRule::default()),
        Box::new(GapAheadHighRule::default()),
        Box::new(GapBehindMediumRule::default()),
        Box::new(GapBehindHighRule::default()),

        // Pace
        Box::new(PersonalBestRule),
        Box::new(PaceDroppingRule),
        Box::new(SectorDeltaRule),
        Box::new(SessionBestOvertakenRule),
        Box::new(ClassAheadSlowerRule),
        Box::new(ClassAheadFasterRule),
        Box::new(ClassBehindFasterRule),
        Box::new(ClassBehindSlowerRule),
        Box::new(ClassBestLapRule),
        Box::new(ClassPaceBriefRule),
        Box::new(SessionBestPaceRule),

        // Tires
        Box::new(TireTempsOutOfRangeRule),
        Box::new(TireTempsInRangeRule),
        Box::new(TireWear50Rule),
        Box::new(TireWear75Rule),
        Box::new(TireWear90Rule),

        // Weather
        Box::new(WeatherBriefingRule),
        Box::new(RainStartingRule),
        Box::new(RainClearingRule),
        Box::new(TrackDryingRule),
        Box::new(RainEscalationRule),
        Box::new(AmbientTempChangeRule::new()),
        Box::new(TrackTempChangeRule::new()),
        Box::new(RainForecastWarningRule::ten_min()),
        Box::new(RainForecastWarningRule::five_min()),
    ]
}
