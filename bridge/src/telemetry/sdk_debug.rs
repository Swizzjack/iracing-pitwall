//! Admin-/Debug-Snapshot: vollständiger Live-Dump aller SDK-Daten.
//!
//! Anders als `TelemetrySnapshot` (kuratierte Whitelist) liefert dieser Snapshot
//! ALLES, was die SDK bereitstellt — jede Variable, das vollständige rohe
//! Session-YAML und die Header-Diagnose. Nur für die versteckte Admin-Ansicht
//! gedacht (siehe `dashboard/src/features/sdk-debug`); wird nirgends persistiert
//! und nur gebaut/gesendet, solange das Panel geöffnet ist.

use serde::Serialize;
use ts_rs::TS;

use crate::iracing_sdk::header::HeaderStatus;
use crate::iracing_sdk::yaml::decode_raw;
use crate::iracing_sdk::IRacingClient;

/// Eine einzelne SDK-Variable mit aktuellem Wert, formatiert für die Anzeige.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct VarDump {
    pub name: String,
    pub desc: String,
    pub unit: String,
    /// "Float", "Int", "Bool", … (Debug-Repräsentation von `VarType`).
    pub var_type: String,
    pub count: usize,
    /// Ein Eintrag pro Array-Element; leer, falls die Variable aktuell
    /// außerhalb des Frame-Puffers liegt (z.B. unmittelbar nach dem Connect).
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct SdkDebugSnapshot {
    pub header: HeaderStatus,
    pub vars: Vec<VarDump>,
    /// Vollständiges, ungefiltertes Session-YAML als Text (Anzeige-only,
    /// wird nicht erneut geparst — siehe `iracing_sdk::yaml::decode_raw`).
    pub session_yaml_raw: String,
}

impl SdkDebugSnapshot {
    pub fn build(client: &IRacingClient) -> Self {
        Self {
            header: HeaderStatus::from_header(client.header()),
            vars: client.dump_all_vars(),
            session_yaml_raw: decode_raw(client.session_info_bytes()),
        }
    }
}
