//! YAML SessionInfo: ISO-8859-1 decode + serde_yaml parse.
//!
//! Wichtig: encoding_rs::WINDOWS_1252 ist eine strikte Obermenge von
//! ISO-8859-1 für den druckbaren Bereich; 0x80-0x9F werden für unsere
//! Zwecke (Fahrernamen, Track-Namen) nicht benötigt.

use crate::error::{BridgeError, Result};
use crate::iracing_sdk::types::SessionInfoYaml;

pub fn decode_and_parse(raw: &[u8]) -> Result<SessionInfoYaml> {
    let (decoded, _enc, had_errors) = encoding_rs::WINDOWS_1252.decode(raw);
    if had_errors {
        log::warn!("YAML decode had replacement errors (non-ISO-8859-1 bytes encountered)");
    }
    let trimmed = decoded.trim_end_matches('\0');
    serde_yaml::from_str::<SessionInfoYaml>(trimmed)
        .map_err(|e| BridgeError::YamlParse(e.to_string()))
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
}
