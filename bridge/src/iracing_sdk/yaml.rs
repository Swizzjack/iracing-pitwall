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
    serde_yaml::from_str::<SessionInfoYaml>(&decoded)
        .map_err(|e| BridgeError::YamlParse(e.to_string()))
}
