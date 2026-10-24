//! Clause-bound scenarios, terminal decisions, and append-only evidence records.

pub mod action;
pub mod actor;
pub mod case;
pub mod definition;
pub mod playground;
pub mod playground_ledger;
pub mod replay;
pub mod timeline;
pub mod trial;

pub use action::{encode, ActionPlan, EncodedAction, Key, KeyAction, Modifiers, Resolution, Step};
pub use actor::{
    run_actor, Actor, ActorDecision, ExpectActor, ExpectStep, LoopConfig, LoopReport, Observation,
    ScriptedActor,
};
pub use case::{Case, CheckSpec, Provenance, Reward};
pub use definition::*;
pub use playground_ledger::{ObserverClaimRecord, ReviewClaimRecord};
pub use replay::{replay, replay_with_mode, Divergence, MatchMode, ReplayReport, ReplayStep};
pub use timeline::{Event, Timeline, VerificationProvenance};
pub use trial::{Trial, TrialSet};
