//! iRacing Shared-Memory SDK reader.
//!
//! Architektur:
//! - `header`:      irsdk_header-Layout und Parser
//! - `var_header`:  Variable-Descriptor-Array, Indexing
//! - `reader`:      MMF öffnen, Triple-Buffer-Read-Loop
//! - `events`:      WaitForSingleObject auf Local\IRSDKDataValidEvent
//! - `yaml`:        ISO-8859-1 decode + serde_yaml parse
//! - `types`:       SessionInfo, DriverInfo, ResultsPositions (ts-rs derive)
//! - `broadcast`:   PostMessageW-Wrapper für Pit/Camera/Replay (später)

pub mod broadcast;
pub mod events;
pub mod header;
pub mod reader;
pub mod types;
pub mod var_header;
pub mod yaml;

// Re-export für öffentliche API. `allow(unused_imports)`, weil
// main.rs den Client erst nach Implementation nutzt.
#[allow(unused_imports)]
pub use reader::IRacingClient;
