//! Weather rules — rain, temperature changes, and proactive weather briefings.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::{EngineerState, SessionPhase};

const RAIN_THRESHOLD: f32 = 0.1;
const HEAVY_RAIN: f32 = 0.5;
const DRYING_RANGE_LO: f32 = 0.05;
const DRYING_RANGE_HI: f32 = 0.3;
const TEMP_CHANGE_C: f32 = 2.0;

/// Proactive weather briefing on car entry or pit-lane exit.
/// Fires once per edge so the driver always hears conditions when going out on track.
pub struct WeatherBriefingRule;

pub struct RainStartingRule;
pub struct RainClearingRule;
pub struct TrackDryingRule;
pub struct RainEscalationRule;

pub struct AmbientTempChangeRule {
    baseline: AtomicU32,
    initialized: AtomicBool,
}

pub struct TrackTempChangeRule {
    baseline: AtomicU32,
    initialized: AtomicBool,
}

/// Fires N minutes before rain arrives (if forecast data available).
/// In iRacing V1, forecast is empty, so this rule never fires — left for future use.
pub struct RainForecastWarningRule {
    minutes: u32,
}

impl RainForecastWarningRule {
    pub fn ten_min() -> Self { Self { minutes: 10 } }
    pub fn five_min() -> Self { Self { minutes: 5 } }
}

impl AmbientTempChangeRule {
    pub fn new() -> Self {
        Self {
            baseline: AtomicU32::new(0),
            initialized: AtomicBool::new(false),
        }
    }
}

impl TrackTempChangeRule {
    pub fn new() -> Self {
        Self {
            baseline: AtomicU32::new(0),
            initialized: AtomicBool::new(false),
        }
    }
}

fn f32_to_bits(v: f32) -> u32 { v.to_bits() }
fn bits_to_f32(v: u32) -> f32 { f32::from_bits(v) }

impl Rule for RainStartingRule {
    fn id(&self) -> &'static str { "rain_starting" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(300) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        let now_wet = current.rain_intensity >= RAIN_THRESHOLD;
        let was_wet = prev.rain_intensity >= RAIN_THRESHOLD;
        if now_wet && !was_wet {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "rain_starting",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for RainClearingRule {
    fn id(&self) -> &'static str { "rain_clearing" }
    fn priority(&self) -> Priority { Priority::High }
    fn cooldown(&self) -> Duration { Duration::from_secs(300) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        let now_dry = current.rain_intensity < RAIN_THRESHOLD;
        let was_wet = prev.rain_intensity >= RAIN_THRESHOLD;
        if now_dry && was_wet {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "rain_clearing",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for TrackDryingRule {
    fn id(&self) -> &'static str { "track_drying" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(240) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        let in_drying = current.rain_intensity > DRYING_RANGE_LO
            && current.rain_intensity < DRYING_RANGE_HI;
        let was_wetter = prev.rain_intensity >= DRYING_RANGE_HI;
        if in_drying && was_wetter {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "track_drying",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for RainEscalationRule {
    fn id(&self) -> &'static str { "rain_escalation" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(300) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::ALL }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;
        if current.rain_intensity >= HEAVY_RAIN && prev.rain_intensity < HEAVY_RAIN {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "rain_escalation",
                params: TemplateParams::new(),
            })
        } else {
            None
        }
    }
}

impl Rule for AmbientTempChangeRule {
    fn id(&self) -> &'static str { "ambient_temp_change" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(600) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let temp = current.ambient_temp_c;
        if !self.initialized.load(Ordering::Relaxed) {
            self.baseline.store(f32_to_bits(temp), Ordering::Relaxed);
            self.initialized.store(true, Ordering::Relaxed);
            return None;
        }
        let baseline = bits_to_f32(self.baseline.load(Ordering::Relaxed));
        if (temp - baseline).abs() >= TEMP_CHANGE_C {
            self.baseline.store(f32_to_bits(temp), Ordering::Relaxed);
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "ambient_temp_change",
                params: TemplateParams::new().set("temp", format!("{:.0}", temp)),
            })
        } else {
            None
        }
    }
}

impl Rule for TrackTempChangeRule {
    fn id(&self) -> &'static str { "track_temp_change" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(600) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let temp = current.track_temp_c;
        if !self.initialized.load(Ordering::Relaxed) {
            self.baseline.store(f32_to_bits(temp), Ordering::Relaxed);
            self.initialized.store(true, Ordering::Relaxed);
            return None;
        }
        let baseline = bits_to_f32(self.baseline.load(Ordering::Relaxed));
        if (temp - baseline).abs() >= TEMP_CHANGE_C {
            self.baseline.store(f32_to_bits(temp), Ordering::Relaxed);
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "track_temp_change",
                params: TemplateParams::new().set("temp", format!("{:.0}", temp)),
            })
        } else {
            None
        }
    }
}

impl Rule for RainForecastWarningRule {
    fn id(&self) -> &'static str { "rain_forecast" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(600) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, _current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        // iRacing V1: no forecast data available; rule is a no-op placeholder
        None
    }
}

impl Rule for WeatherBriefingRule {
    fn id(&self) -> &'static str { "weather_briefing" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(60) }
    fn session_mask(&self) -> SessionMask { SessionMask::ALL }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&self, current: &EngineerState, prev: Option<&EngineerState>) -> Option<RuleEvent> {
        let prev = prev?;

        // Fire on car-entry rising edge or pit-lane exit rising edge.
        // Both edges occur when the driver is NOT pit-muted (car-entry = before driving;
        // pit-lane exit = first non-pit tick). No need to bypass the pit-lane mute.
        let car_entry_edge = prev.session_phase != SessionPhase::GettingInCar
            && current.session_phase == SessionPhase::GettingInCar;
        let pit_exit_edge = prev.in_pit && !current.in_pit;

        if !car_entry_edge && !pit_exit_edge {
            return None;
        }

        let condition = if current.rain_intensity >= 0.3 {
            "wet"
        } else if current.rain_intensity >= 0.05 {
            "damp"
        } else {
            "dry"
        };

        let mut params = TemplateParams::new()
            .set("condition", condition.to_string())
            .set("track_temp", format!("{:.0}", current.track_temp_c))
            .set("air_temp", format!("{:.0}", current.ambient_temp_c));

        // Include wind if available and meaningful (> 1 m/s ≈ 3.6 km/h)
        if let Some(ws) = current.wind_speed_ms {
            if ws > 1.0 {
                params = params.set("wind", format!("{:.0} km/h", ws * 3.6));
            }
        }

        Some(RuleEvent {
            rule_id: self.id(),
            priority: self.priority(),
            template_key: "weather_briefing",
            params,
        })
    }
}
