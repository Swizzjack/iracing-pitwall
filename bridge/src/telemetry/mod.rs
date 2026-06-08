//! Normalisierte Telemetrie-Datenmodelle für das Dashboard.
//!
//! - `snapshot`: 60-Hz-Stream (Pedals, Speed, Gear, Fuel, Tires, ...)
//! - `standings`: 4-Hz-Stream (CarIdx-basierte Rangliste, Gaps)
//! - `track_recorder`: VelocityXY-Integration, Disk-Cache
//! - `track_map`: 15-Hz TrackMap-Snapshot (Autopos. + Streckenform)

pub mod finish_tracker;
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
