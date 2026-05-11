//! Shared thresholds for the Phase C quality gates.
//!
//! Concept, audience, character, and structure critics all use the
//! same composite-and-floor pair. Defining them once here keeps the
//! four critic types from drifting.
//!
//! Severity grading for findings lives on
//! [`crate::validator::Severity`] (the same enum the export-gate
//! validators already use). The Phase C critics opt into tolerant
//! deserialisation via
//! [`crate::validator::deserialize_severity_tolerant`].

/// Per-axis floor every Phase C critic enforces. An axis below this
/// fails the gate regardless of how high the composite is.
pub const AXIS_FLOOR: f32 = 7.0;

/// Composite (mean of axes) threshold every Phase C critic enforces.
/// A proposal whose composite is below this fails even when every
/// individual axis is at or above the floor.
pub const COMPOSITE_THRESHOLD: f32 = 8.5;
