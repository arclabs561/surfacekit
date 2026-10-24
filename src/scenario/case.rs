//! A regression case as a quadruple: init, act, check, teardown.
//!
//! The shape is deliberate. A case that only records the final assertion cannot
//! reproduce a failure; a case that records a live URL or model cannot claim
//! replay. Non-reproducible dependencies are named, not hidden.

use crate::scenario::action::{ActionPlan, Resolution, Step};
use crate::scenario::timeline::{Event, Timeline, VerificationProvenance};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckSpec {
    pub name: String,
    /// Substring that must be visible on the rendered screen.
    pub contains: String,
    pub provenance: VerificationProvenance,
}

/// Where a case came from. Absence is unknown, not clean.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub harness: Option<String>,
    pub model: Option<String>,
    pub prompt_hash: Option<String>,
    pub environment: Option<String>,
}

/// One reward record per trial. Cheap deterministic signal and judged signal
/// stay separate so an optimizer can weight them independently.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reward {
    pub passed: bool,
    pub score: Option<f64>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<u64>,
    pub action_similarity: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub id: String,
    pub init: Vec<Step>,
    pub act: Vec<Step>,
    pub checks: Vec<CheckSpec>,
    pub teardown: Vec<Step>,
    /// Live services, mutable sites, or live models that make a byte-for-byte
    /// replay a claim rather than a fact.
    pub live_dependencies: Vec<String>,
    #[serde(default)]
    pub provenance: Provenance,
    #[serde(default)]
    pub reward: Option<Reward>,
}

impl Case {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            init: Vec::new(),
            act: Vec::new(),
            checks: Vec::new(),
            teardown: Vec::new(),
            live_dependencies: Vec::new(),
            provenance: Provenance::default(),
            reward: None,
        }
    }

    #[must_use]
    pub fn is_reproducible(&self) -> bool {
        self.live_dependencies.is_empty()
    }

    /// The recorded actions as one plan, in append order.
    #[must_use]
    pub fn action_plan(&self) -> ActionPlan {
        ActionPlan::new(Resolution::Plan, self.act.clone())
    }

    /// Promote the actions in a timeline. Checks and provenance are supplied
    /// by the caller; a run without an oracle is not a case.
    #[must_use]
    pub fn from_timeline(
        id: impl Into<String>,
        timeline: &Timeline,
        checks: Vec<CheckSpec>,
    ) -> Self {
        let act = timeline
            .events()
            .iter()
            .filter_map(|event| match event {
                Event::Action { plan, .. } => Some(Step::Plan {
                    resolution: plan.resolution,
                    steps: plan.steps.clone(),
                }),
                _ => None,
            })
            .collect();
        Self {
            id: id.into(),
            init: Vec::new(),
            act,
            checks,
            teardown: Vec::new(),
            live_dependencies: Vec::new(),
            provenance: Provenance::default(),
            reward: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::action::{ActionPlan, Resolution};

    #[test]
    fn a_case_names_its_live_dependencies_instead_of_claiming_replay() {
        let case = Case {
            live_dependencies: vec!["https://example.com".into()],
            ..Case::new("live-site")
        };
        assert!(!case.is_reproducible());

        let local = Case::new("local-tui");
        assert!(local.is_reproducible());
        assert_eq!(local.act, Vec::<Step>::new());
        assert_eq!(
            ActionPlan::new(Resolution::Plan, local.act.clone())
                .steps
                .len(),
            0
        );
    }
}
