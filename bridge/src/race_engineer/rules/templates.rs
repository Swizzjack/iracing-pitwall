//! Template registry for race engineer radio calls.
//!
//! Each template key maps to several variant strings. The `render()` method
//! picks a pseudo-random variant, filters out any variant whose `{placeholder}`
//! parameters are not satisfied, and performs string substitution.

use std::collections::HashMap;

use crate::race_engineer::rules::TemplateParams;

pub struct TemplateRegistry {
    templates: HashMap<&'static str, &'static [&'static str]>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        let mut t: HashMap<&'static str, &'static [&'static str]> = HashMap::new();

        // --- Critical ---
        t.insert("meatball_flag", &[
            "Meatball flag! You have a mechanical issue. Pit this lap.",
            "Black and orange flag for you. Come in, pit this lap.",
            "Meatball flag shown. Box this lap for repairs.",
            "You have a meatball flag. Bring it in immediately.",
        ]);
        t.insert("red_flag", &[
            "Red flag! Red flag! Slow down and hold position.",
            "Red flag out. Reduce speed immediately.",
            "Red flag, red flag. Come back to the pits safely.",
            "Red flag shown. Box slowly on the warm-up lap.",
        ]);
        t.insert("fuel_critical_box", &[
            "Box this lap, box this lap. Fuel critical.",
            "You need to pit now. Fuel is critical, box box box.",
            "Pit this lap. Fuel at critical level.",
            "Box box box. You are on critical fuel.",
        ]);
        t.insert("drivethrough_penalty", &[
            "You have a drive-through penalty. Box box box.",
            "Drive-through penalty confirmed. Serve it this lap.",
            "Penalty: drive-through. Pit lane this lap.",
        ]);

        // --- High / Critical — Incidents ---
        t.insert("incident_update_no_limit", &[
            "Incident. {count} points on the board.",
            "{delta} point incident. {count} total.",
            "Copy that. {count} incident points total.",
            "Noted, {count} points on the board.",
        ]);
        t.insert("incident_update", &[
            "Incident. {count} of {limit} — {remaining} left.",
            "{delta} points. {count} of {limit}, {remaining} remaining.",
            "Copy. {count} points, {remaining} remaining before the limit.",
            "{count} of {limit} incident points. {remaining} to go.",
        ]);
        t.insert("incident_warning", &[
            "Watch it! {count} of {limit} — only {remaining} points left.",
            "Caution: {count} of {limit} incidents. {remaining} more and you're out.",
            "Getting tight: {count} points, {remaining} remaining. Drive clean.",
        ]);
        t.insert("incident_critical", &[
            "Warning! {count} of {limit} — only {remaining} left before exclusion. Drive clean!",
            "Critical! You have {remaining} incident points left. {count} of {limit}.",
            "One more incident and you're done. {count} of {limit}. Be very careful.",
        ]);

        // --- High ---
        t.insert("yellow_flag_sector", &[
            "Yellow flag in sector {sector}. Stay wide and lift.",
            "Caution, yellow in sector {sector}. Be careful.",
            "Sector {sector} yellow flag. Reduce speed and stay wide.",
            "Yellow in sector {sector}. Slow down, keep it clean.",
        ]);
        t.insert("debris_flag", &[
            "Debris on track. Stay alert and stay wide.",
            "Debris flag. Watch for hazards on the racing line.",
            "Debris on circuit. Lift if you see anything on track.",
            "Debris warning. Keep your eyes up.",
        ]);
        t.insert("blue_flag", &[
            "Blue flag! Let the leaders through.",
            "Blue flag shown. Move over for the class leaders.",
            "Blue flag. Let them by cleanly.",
            "Blue flag, {driver_name} — let the leaders through.",
        ]);
        t.insert("damage_reported", &[
            "Damage reported: {damage_type} damage. Check your balance.",
            "We have {damage_type} damage. Monitor carefully.",
            "Car shows {damage_type} damage. Adjust your driving.",
        ]);
        t.insert("fuel_low", &[
            "Fuel low. Approximately {laps} laps remaining on current fuel.",
            "Watch your fuel. About {laps} laps left.",
            "Fuel is getting low. {laps} laps on current fuel.",
            "Low fuel warning. You have {laps} laps of fuel.",
        ]);
        t.insert("consider_pit", &[
            "Consider boxing in the next few laps. Fuel window is open.",
            "Good time to pit. Fuel window now open.",
            "Pit window open. Suggest boxing in the next two laps.",
        ]);
        t.insert("tire_wear_90", &[
            "Tyre wear at 90 percent. Box as soon as possible.",
            "Tyres are at 90 percent wear. Plan your pit stop now.",
            "Critical tyre wear. Consider pitting this lap.",
        ]);
        t.insert("rain_starting", &[
            "Rain starting. Intermediate or wet tyres may be needed.",
            "Rain coming down. Consider tyre strategy.",
            "It's raining. Monitor track conditions.",
            "Rain is starting. Think about a tyre change.",
        ]);
        t.insert("rain_clearing", &[
            "Rain is clearing. Track is starting to dry.",
            "Rain easing off. Slick tyres may be an option.",
            "Conditions improving. Track drying.",
        ]);
        t.insert("session_best_overtaken", &[
            "Your session best has been beaten. Push harder.",
            "Fastest lap in class has been taken. Respond.",
            "New session best set. You need to respond.",
        ]);

        // --- Info ---
        t.insert("green_flag", &[
            "Green flag. Push now.",
            "Green green green. Let's go.",
            "All clear, green flag. Push it.",
            "Green flag. Good luck, {driver_name}.",
        ]);
        t.insert("position_gained", &[
            "Position gained. You are now P{position}.",
            "Up to P{position}. Good work.",
            "Nice pass, you're P{position} now.",
            "P{position}. Well done, {driver_name}.",
        ]);
        t.insert("position_lost", &[
            "Position lost. You are now P{position}.",
            "Dropped to P{position}. Stay focused.",
            "We've dropped a position. P{position} now.",
        ]);
        t.insert("gap_ahead", &[
            "Gap to car ahead: {gap} seconds, {trend}.",
            "Car ahead {gap} seconds, {trend}.",
            "{gap} seconds to P{position} ahead. {trend}.",
        ]);
        t.insert("gap_behind", &[
            "Gap behind: {gap} seconds, {trend}.",
            "Car behind {gap} seconds, {trend}.",
            "P{position} behind is {gap} seconds. {trend}.",
        ]);
        t.insert("personal_best", &[
            "New personal best, {lap_time}. Keep it up.",
            "Personal best! {lap_time}. Great lap.",
            "That's a personal best. Good job, {driver_name}.",
            "New PB. Excellent lap.",
        ]);
        t.insert("pace_dropping", &[
            "Your pace is dropping. Focus on the process.",
            "Lap times are slipping. Stay with it.",
            "We're losing pace. Check tyre temperatures.",
        ]);
        t.insert("sector_delta", &[
            "Sector {sector}: {delta} seconds to your best.",
            "Sector {sector} delta: {delta}.",
        ]);
        t.insert("five_minutes", &[
            "Five minutes remaining.",
            "Five minutes left in the session.",
            "Five to go.",
        ]);
        t.insert("last_lap", &[
            "Last lap.",
            "This is the final lap.",
            "One lap to go, {driver_name}. Make it count.",
            "Final lap. Everything you've got.",
        ]);
        t.insert("race_finished", &[
            "Race finished. Well done.",
            "Chequered flag. Great race, {driver_name}.",
            "That's the chequered. Race over.",
            "Race complete. Bring it home.",
        ]);
        t.insert("tire_temps_hot", &[
            "Tyre temperatures running hot. Watch your speed in the slow corners.",
            "Tyres are overheating. Manage them through this section.",
            "Hot tyres. Adjust your driving style.",
        ]);
        t.insert("tire_temps_cold", &[
            "Tyres coming back into the working range.",
            "Tyre temperatures returning to normal.",
            "Tyres are back in range.",
        ]);
        t.insert("tire_wear_50", &[
            "50 percent tyre wear.",
            "Halfway on the tyres.",
            "Tyres at 50 percent.",
        ]);
        t.insert("tire_wear_75", &[
            "Tyre wear at 75 percent. Start planning your strategy.",
            "Three-quarter tyre wear. Think about the pit window.",
            "75 percent wear on the tyres.",
        ]);
        t.insert("track_drying", &[
            "Track is drying. Consider slick tyres.",
            "The track is coming in. Slicks may be faster.",
            "Dry line appearing. Monitor conditions.",
        ]);
        t.insert("rain_escalation", &[
            "Heavy rain. Be careful in the low-grip areas.",
            "Rain is intensifying. Take care.",
            "Conditions worsening. Reduce your pace.",
        ]);
        t.insert("ambient_temp_change", &[
            "Ambient temperature has changed to {temp} degrees.",
            "Air temperature now {temp} Celsius.",
        ]);
        t.insert("track_temp_change", &[
            "Track temperature has changed to {temp} degrees.",
            "Track temp now {temp} Celsius. Adjust tyre management.",
        ]);
        t.insert("rain_forecast", &[
            "Rain forecast in approximately {minutes} minutes. Plan ahead.",
            "Weather radar shows rain in {minutes} minutes.",
            "Rain expected in {minutes} minutes. Consider tyre strategy.",
        ]);
        t.insert("class_best_lap", &[
            "You have the fastest lap in class. Defend it.",
            "Class fastest lap is yours. Keep pushing.",
            "P1 on pace in class. Well done.",
        ]);
        t.insert("class_ahead_slower", &[
            "Class car ahead is slower by {gap} per lap. Close them down.",
            "You're faster than the car ahead. {gap} seconds a lap.",
        ]);
        t.insert("class_ahead_faster", &[
            "Class car ahead is faster by {gap}. You need to push.",
            "Gap ahead is opening. {gap} seconds a lap faster.",
        ]);
        t.insert("class_behind_faster", &[
            "Class car behind is {gap} seconds per lap faster. Manage the gap.",
            "Car behind is closing. {gap} quicker per lap.",
        ]);
        t.insert("class_behind_slower", &[
            "Class car behind is slower. You're building the gap.",
            "Car behind is losing pace. Good news.",
        ]);
        t.insert("pit_exit_briefing", &[
            "Pit exit. Track is {track_condition}. Tyres are {tyre_status}. Track {track_temp}, air {air_temp} Celsius.",
            "Out of the pits. Track: {track_condition}. Tyre status: {tyre_status}. Air {temp} degrees.",
            "Pit exit. Warm the tyres up. Track is {track_condition}.",
        ]);

        // --- Pace briefs (practice/qualifying/race) ---
        t.insert("class_pace_brief", &[
            "Class best is {class_best}. You're {delta} seconds off the fastest in class.",
            "Quickest class lap is {class_best}. Your delta: {delta}.",
            "Fastest in your class: {class_best}. You're {delta} off. Field average last lap {field_avg}.",
        ]);
        t.insert("session_best_pace", &[
            "Best in class so far: {class_best}.",
            "Benchmark lap is {class_best}.",
            "Session best in class: {class_best}. Match it.",
        ]);

        // --- Weather briefing ---
        t.insert("weather_briefing", &[
            "Track is {condition}. Track temp {track_temp} degrees, air {air_temp}.",
            "Conditions: {condition}. {track_temp} on track, {air_temp} ambient. Wind {wind}.",
            "Weather check: {condition} track. {track_temp} degrees surface, {air_temp} air.",
        ]);

        Self { templates: t }
    }

    /// Render a template to a speech string.
    ///
    /// Selects a pseudo-random variant (using a simple modulo seed),
    /// filters variants with unsatisfied placeholders, and substitutes.
    pub fn render(&self, key: &str, params: &TemplateParams, seed: usize) -> Option<String> {
        let variants = self.templates.get(key)?;

        // Filter to variants whose placeholders are all satisfied
        let mut eligible: Vec<&str> = variants
            .iter()
            .copied()
            .filter(|v| placeholders_satisfied(v, params))
            .collect();

        if eligible.is_empty() {
            // Fall back to all variants (some placeholder may be missing)
            eligible = variants.to_vec();
        }

        let chosen = eligible[seed % eligible.len()];
        let mut text = chosen.to_string();

        // Substitute all placeholders
        for key in params.keys() {
            if let Some(val) = params.get(key) {
                text = text.replace(&format!("{{{key}}}"), val);
            }
        }

        log::debug!("Template render: key={key} → {text:?}");
        Some(text)
    }
}

fn placeholders_satisfied(template: &str, params: &TemplateParams) -> bool {
    let mut i = 0;
    let bytes = template.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = template[i..].find('}') {
                let placeholder = &template[i + 1..i + end];
                if params.get(placeholder).is_none() {
                    return false;
                }
                i += end + 1;
                continue;
            }
        }
        i += 1;
    }
    true
}
