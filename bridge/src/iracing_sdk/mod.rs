//! iRacing shared-memory SDK reader.
//!
//! Architecture:
//! - `header`:      irsdk_header layout and parser
//! - `var_header`:  variable-descriptor array, indexing
//! - `reader`:      open the MMF, event wait, triple-buffer read loop
//! - `yaml`:        ISO-8859-1 decode + serde_yaml parse
//! - `types`:       SessionInfo, DriverInfo, ResultsPositions (ts-rs derive)

pub mod header;
pub mod reader;
pub mod synthetic_id;
pub mod types;
pub mod var_header;
pub mod yaml;

pub use reader::IRacingClient;
