//! iRacing shared-memory SDK reader.
//!
//! Architecture:
//! - `header`:      irsdk_header layout and parser
//! - `var_header`:  variable-descriptor array, indexing
//! - `reader`:      open the MMF, triple-buffer read loop
//! - `events`:      WaitForSingleObject on Local\IRSDKDataValidEvent
//! - `yaml`:        ISO-8859-1 decode + serde_yaml parse
//! - `types`:       SessionInfo, DriverInfo, ResultsPositions (ts-rs derive)
//! - `broadcast`:   PostMessageW wrapper for Pit/Camera/Replay (later)

pub mod broadcast;
pub mod events;
pub mod header;
pub mod reader;
pub mod synthetic_id;
pub mod types;
pub mod var_header;
pub mod yaml;

// Re-export for the public API. `allow(unused_imports)` because
// main.rs only uses the client once the implementation is in place.
#[allow(unused_imports)]
pub use reader::IRacingClient;
