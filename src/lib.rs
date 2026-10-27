//! Surface-neutral observation and evidence packets for agentic testing.
//!
//! The crate has four internal layers:
//!
//! - [`contract`] — surface, observation, evidence, and claim vocabulary.
//! - [`notes`] — a critic's dated, append-only running notes (part of the
//!   contract: they travel in the packet).
//! - [`term`] — real PTY, VT100 screen, workspace diff, and recording.
//! - [`scenario`] — clause-bound scenarios and append-only evidence records.
//! - [`judge`] — deterministic checks and a provider-neutral judge seam.
//!
//! `contract` is always available. `term`, `scenario`, and `judge` are feature
//! gated so a consumer that only needs the vocabulary does not compile a PTY.
//!
//! The layers are modules rather than separate packages so the crate can be
//! published once; a published crate cannot depend on unpublished path crates.

pub mod contract;
#[cfg(feature = "judge")]
pub mod judge;
pub mod notes;
#[cfg(feature = "scenario")]
pub mod scenario;
#[cfg(feature = "term")]
pub mod term;

pub use contract::{
    Claim, EvidenceKind, Observation, Surface, SurfacePacket, CURRENT_SCHEMA_VERSION,
    OUTPUT_RECOVERY_ROOM, SUPPORTED_SCHEMA_VERSIONS,
};
pub use notes::{Note, NoteDraft, NoteKind, Notebook};
