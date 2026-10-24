//! Surface-neutral observation and evidence packets for agentic testing.
//!
//! The crate has four internal layers:
//!
//! - [`contract`] — surface, observation, evidence, and claim vocabulary.
//! - [`term`] — real PTY, VT100 screen, workspace diff, and recording.
//! - [`scenario`] — clause-bound scenarios and append-only evidence records.
//! - [`judge`] — deterministic checks and a provider-neutral judge seam.
//!
//! The layers are modules rather than separate packages so the crate can be
//! published once; a published crate cannot depend on unpublished path crates.

pub mod contract;
pub mod judge;
pub mod scenario;
pub mod term;

pub use contract::{
    Claim, EvidenceKind, Observation, Surface, SurfacePacket, CURRENT_SCHEMA_VERSION,
    OUTPUT_RECOVERY_ROOM,
};
