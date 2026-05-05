//! Normalisierte Telemetrie-Datenmodelle für das Dashboard.
//!
//! - `snapshot`: 60-Hz-Stream (Pedals, Speed, Gear, Fuel, Tires, ...)
//! - `standings`: 4-Hz-Stream (CarIdx-basierte Rangliste, Gaps)

pub mod pit_tracker;
pub mod snapshot;
pub mod standings;

pub use snapshot::TelemetrySnapshot;
pub use standings::StandingsSnapshot;
