//! Bounded live critique.
//!
//! The actor loop owns the latency budget. A critic answers from the same
//! screen the actor acted on, so the critique is about the decision rather
//! than a screen the action has not produced yet. The loop measures the call
//! and records over-budget critiques instead of dropping them; it cannot
//! preempt a synchronous critic. Preemption is a separate gate.

use crate::scenario::action::ActionPlan;

/// What a critic may read. Borrowed so the loop keeps ownership of the
/// observation and the plan it just recorded.
#[derive(Debug, Clone, Copy)]
pub struct CritiqueContext<'a> {
    pub screen: &'a str,
    pub action: Option<&'a ActionPlan>,
    pub elapsed_ms: u64,
}

/// The loop's latency contract. The loop measures; it does not enforce by
/// interrupting a critic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CritiqueBudget {
    pub max_latency_ms: u64,
}

impl Default for CritiqueBudget {
    fn default() -> Self {
        Self {
            max_latency_ms: 250,
        }
    }
}

/// A live critic. Returning `None` means "no opinion", and the loop records
/// nothing for that action.
pub trait Critic {
    fn critique(&mut self, context: CritiqueContext<'_>) -> Option<String>;
}
