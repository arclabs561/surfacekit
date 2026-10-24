//! Deterministic checks and provider-neutral judge adapters.
//!
//! The judge layer never owns a provider client. Callers implement
//! [`model::JudgeModel`] and supply the observer prompt as data, so provider
//! credentials, endpoints, and evaluation methodology stay with the consumer.

pub mod checks;
pub mod memory;
pub mod model;
pub mod pairwise;
pub mod recursive_observer;

pub use checks::deterministic_checks;
pub use memory::{ConversationMemory, TurnSummary};
pub use model::{JudgeModel, JudgeOutcome, Persona};
pub use pairwise::{counterbalance, Agreement, Counterbalanced, PairwiseJudge, Side, Verdict};
pub use recursive_observer::{observe, RecursiveReview};
