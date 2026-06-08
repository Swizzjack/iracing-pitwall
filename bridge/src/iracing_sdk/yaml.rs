//! YAML SessionInfo: ISO-8859-1 decode + serde_yaml parse.
//!
//! Wichtig: encoding_rs::WINDOWS_1252 ist eine strikte Obermenge von
//! ISO-8859-1 für den druckbaren Bereich; 0x80-0x9F werden für unsere
//! Zwecke (Fahrernamen, Track-Namen) nicht benötigt.
//!
//! iRacing erzeugt in Abschnitten wie CarSetup, CameraInfo, RadioInfo
//! manchmal nicht-valides YAML (z.B. Fahrzeugnamen mit Doppelpunkten,
//! unkorrekte Block-Nodes). Da serde_yaml das gesamte Dokument parst,
//! filtern wir vorab alle nicht benötigten Top-Level-Sektionen heraus.

use crate::error::{BridgeError, Result};
use crate::iracing_sdk::types::SessionInfoYaml;

const KEEP: &[&str] = &["WeekendInfo", "SessionInfo", "DriverInfo", "SplitTimeInfo"];

/// Behält nur die in `keep` aufgeführten Top-Level-Sektionen des YAML-Dokuments.
/// Eine Top-Level-Sektion beginnt mit einer Zeile ohne führende Whitespace-Zeichen,
/// die nicht mit `#` oder `-` startet und einen `:` enthält.
fn keep_sections(yaml: &str, keep: &[&str]) -> String {
    let mut out = String::with_capacity(yaml.len() / 2);
    let mut in_section = false;

    for line in yaml.lines() {
        let first = line.chars().next();
        let is_top_level = matches!(first, Some(c) if !c.is_whitespace() && c != '#' && c != '-' && c != '.');
        if is_top_level {
            let section_name = line.split(':').next().unwrap_or("").trim();
            in_section = keep.iter().any(|k| *k == section_name);
        }
        if in_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

pub fn decode_and_parse(raw: &[u8]) -> Result<SessionInfoYaml> {
    let filtered = keep_sections(&decode_raw(raw), KEEP);
    serde_yaml::from_str::<SessionInfoYaml>(&filtered)
        .map_err(|e| BridgeError::YamlParse(e.to_string()))
}

/// Decodes the raw session-info bytes (Windows-1252/ISO-8859-1, NUL-padded tail)
/// to a string, WITHOUT filtering any sections. Used by the debug/admin view to
/// show the complete YAML exactly as iRacing emits it (including the sections
/// that are normally stripped because they sometimes contain invalid YAML —
/// see module docs). Display-only; never fed back into a YAML parser.
pub fn decode_raw(raw: &[u8]) -> String {
    let (decoded, _enc, had_errors) = encoding_rs::WINDOWS_1252.decode(raw);
    if had_errors {
        log::warn!("YAML decode had replacement errors (non-ISO-8859-1 bytes encountered)");
    }
    decoded.trim_end_matches('\0').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture lives in bridge/tests/fixtures/ — path relative to this source file.
    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/session_minimal.yaml");

    #[test]
    fn parses_fixture_clean() {
        let info = decode_and_parse(FIXTURE).expect("fixture should parse");
        assert_eq!(info.weekend_info.track_name, "okayama short");
        assert_eq!(info.session_info.sessions.len(), 2);
        assert_eq!(info.driver_info.drivers.len(), 3);
    }

    #[test]
    fn parses_fixture_with_nul_tail() {
        let mut padded = FIXTURE.to_vec();
        padded.extend_from_slice(&[0u8; 1024]);
        let info = decode_and_parse(&padded).expect("NUL-padded fixture should parse");
        assert_eq!(info.weekend_info.track_name, "okayama short");
        assert!(!info.session_info.sessions.is_empty());
        assert!(info.driver_info.drivers.len() >= 2);
    }

    #[test]
    fn ignores_invalid_yaml_in_unknown_sections() {
        // Simulate iRacing emitting broken YAML in CarSetup (colon in unquoted value).
        let yaml = std::str::from_utf8(FIXTURE).unwrap().to_owned()
            + "\nCarSetup:\n Tyres:\n  FrontLeft: Some: Bad: Value\n";
        let info = decode_and_parse(yaml.as_bytes()).expect("broken CarSetup must not abort parse");
        assert_eq!(info.weekend_info.track_name, "okayama short");
    }

    #[test]
    fn keep_sections_only_returns_wanted() {
        let yaml = "WeekendInfo:\n TrackName: foo\nCarSetup:\n bad: colon: here\nDriverInfo:\n DriverCarIdx: 0\n Drivers: []\n";
        let out = keep_sections(yaml, KEEP);
        assert!(out.contains("WeekendInfo"));
        assert!(out.contains("DriverInfo"));
        assert!(!out.contains("CarSetup"));
    }
}
