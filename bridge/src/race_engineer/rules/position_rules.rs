//! Position and gap rules.

use std::time::{Duration, Instant};

use super::{FrequencyMask, Priority, Rule, RuleEvent, SessionMask, TemplateParams};
use crate::race_engineer::state::{EngineerState, SessionPhase};

/// A position change is only announced once it has been held this long.
/// Live positions flicker while cars run side by side, and the TTS callout
/// itself takes a couple of seconds — announcing instantly means the driver
/// hears a position they no longer hold.
const POSITION_DEBOUNCE: Duration = Duration::from_secs(3);

/// Tracks the live position and reports a change only after it has been
/// stable for [`POSITION_DEBOUNCE`].
#[derive(Default)]
struct StablePositionTracker {
    /// Last position accepted as stable (announced or silently adopted).
    stable: Option<u32>,
    /// Candidate position and when it was first seen.
    pending: Option<(u32, Instant)>,
}

impl StablePositionTracker {
    /// Feed the current live position; returns `Some((from, to))` once a new
    /// position has held for the debounce window. The tracker adopts the new
    /// position regardless of direction so gained/lost trackers stay in sync.
    fn update(&mut self, pos: u32) -> Option<(u32, u32)> {
        if pos == 0 {
            self.pending = None;
            return None;
        }
        let Some(stable) = self.stable else {
            self.stable = Some(pos);
            return None;
        };
        if pos == stable {
            self.pending = None;
            return None;
        }
        match self.pending {
            Some((candidate, since)) if candidate == pos => {
                if since.elapsed() >= POSITION_DEBOUNCE {
                    self.pending = None;
                    self.stable = Some(pos);
                    return Some((stable, pos));
                }
                None
            }
            _ => {
                self.pending = Some((pos, Instant::now()));
                None
            }
        }
    }

    fn reset(&mut self) {
        self.stable = None;
        self.pending = None;
    }
}

#[derive(Default)]
pub struct PositionGainedRule {
    tracker: StablePositionTracker,
}

#[derive(Default)]
pub struct PositionLostRule {
    tracker: StablePositionTracker,
}

/// Gap ahead — Medium frequency (longer cooldown).
#[derive(Default)]
pub struct GapAheadMediumRule {
    last_announced_gap: Option<f32>,
}
/// Gap ahead — High frequency (shorter cooldown).
#[derive(Default)]
pub struct GapAheadHighRule {
    last_announced_gap: Option<f32>,
}
/// Gap behind — Medium frequency.
#[derive(Default)]
pub struct GapBehindMediumRule {
    last_announced_gap: Option<f32>,
}
/// Gap behind — High frequency.
#[derive(Default)]
pub struct GapBehindHighRule {
    last_announced_gap: Option<f32>,
}

impl Rule for PositionGainedRule {
    fn id(&self) -> &'static str { "position_gained" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        // Grid/formation shuffles are noise; the reset also re-baselines
        // between sessions so a new race never inherits the old position.
        if current.session_phase != SessionPhase::Racing {
            self.tracker.reset();
            return None;
        }
        let (from, to) = self.tracker.update(current.player_position)?;
        if to < from {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "position_gained",
                params: TemplateParams::new().set("position", to.to_string()),
            })
        } else {
            None
        }
    }
}

impl Rule for PositionLostRule {
    fn id(&self) -> &'static str { "position_lost" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM_AND_UP }

    fn evaluate(&mut self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        if current.session_phase != SessionPhase::Racing {
            self.tracker.reset();
            return None;
        }
        let (from, to) = self.tracker.update(current.player_position)?;
        if to > from {
            Some(RuleEvent {
                rule_id: self.id(),
                priority: self.priority(),
                template_key: "position_lost",
                params: TemplateParams::new().set("position", to.to_string()),
            })
        } else {
            None
        }
    }
}

fn gap_event(
    rule_id: &'static str,
    template_key: &'static str,
    gap: Option<f32>,
    in_pit: bool,
    last_announced_gap: &mut Option<f32>,
) -> Option<RuleEvent> {
    if in_pit {
        return None;
    }
    let gap = gap?;
    // Trend relative to the previous *announcement* — adjacent 10 Hz ticks
    // only differ by milliseconds and would always read "holding steady".
    let trend = match last_announced_gap.replace(gap) {
        Some(prev_gap) if gap - prev_gap < -0.2 => "closing",
        Some(prev_gap) if gap - prev_gap > 0.2 => "pulling away",
        _ => "holding steady",
    };
    Some(RuleEvent {
        rule_id,
        priority: Priority::Info,
        template_key,
        params: TemplateParams::new()
            .set("gap", format!("{:.1}", gap))
            .set("trend", trend.to_string()),
    })
}

impl Rule for GapAheadMediumRule {
    fn id(&self) -> &'static str { "gap_ahead_medium" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(150) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM }

    fn evaluate(&mut self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        gap_event(self.id(), "gap_ahead", current.gap_ahead, current.in_pit, &mut self.last_announced_gap)
    }
}

impl Rule for GapAheadHighRule {
    fn id(&self) -> &'static str { "gap_ahead_high" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        gap_event(self.id(), "gap_ahead", current.gap_ahead, current.in_pit, &mut self.last_announced_gap)
    }
}

impl Rule for GapBehindMediumRule {
    fn id(&self) -> &'static str { "gap_behind_medium" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(150) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::MEDIUM }

    fn evaluate(&mut self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        gap_event(self.id(), "gap_behind", current.gap_behind, current.in_pit, &mut self.last_announced_gap)
    }
}

impl Rule for GapBehindHighRule {
    fn id(&self) -> &'static str { "gap_behind_high" }
    fn priority(&self) -> Priority { Priority::Info }
    fn cooldown(&self) -> Duration { Duration::from_secs(30) }
    fn session_mask(&self) -> SessionMask { SessionMask::RACE }
    fn frequency_mask(&self) -> FrequencyMask { FrequencyMask::HIGH }

    fn evaluate(&mut self, current: &EngineerState, _prev: Option<&EngineerState>) -> Option<RuleEvent> {
        gap_event(self.id(), "gap_behind", current.gap_behind, current.in_pit, &mut self.last_announced_gap)
    }
}
