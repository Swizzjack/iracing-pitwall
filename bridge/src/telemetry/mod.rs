//! Normalized telemetry data models for the dashboard.
//!
//! - `snapshot`: 60 Hz stream (Pedals, Speed, Gear, Fuel, Tires, ...)
//! - `standings`: 4 Hz stream (CarIdx-based ranking, gaps)
//! - `track_recorder`: VelocityXY integration, disk cache
//! - `track_map`: 15 Hz TrackMap snapshot (car positions + track shape)

pub mod finish_tracker;
pub mod p2p_tracker;
pub mod pit_tracker;
pub mod sdk_debug;
pub mod sector_tracker;
pub mod session_transition;
pub mod snapshot;
pub mod standings;
pub mod track_map;
pub mod track_recorder;

pub use sdk_debug::SdkDebugSnapshot;
pub use snapshot::TelemetrySnapshot;
pub use standings::StandingsSnapshot;
pub use track_map::TrackMapSnapshot;
