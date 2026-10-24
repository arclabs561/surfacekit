//! Deterministic re-feed of a session into a fresh emulator.
//!
//! Replay is an assertion, not a video: the caller supplies the expected
//! observable state after each action, and divergence is reported with the
//! first frame that no longer matches.

use crate::contract::TerminalProfile;
use crate::scenario::action::{encode, ActionPlan};
use crate::term::TerminalEmulator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayStep {
    pub at_ms: u64,
    pub action: ActionPlan,
    /// Expected screen state. Interpreted by [`MatchMode`].
    pub expected: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub at_ms: u64,
    pub expected: String,
    pub actual: String,
}

/// How strictly replay compares the observed screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MatchMode {
    /// Do not compare; replay only re-feeds actions.
    Skip,
    /// The expected string must appear; extra output is allowed.
    #[default]
    Contains,
    /// The trimmed screen must equal the expected string exactly.
    Exact,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReplayReport {
    pub steps: usize,
    pub divergences: Vec<Divergence>,
}

impl ReplayReport {
    #[must_use]
    pub fn passed(&self) -> bool {
        self.divergences.is_empty()
    }
}

/// Feed each action into `emulator` and compare with the default
/// [`MatchMode::Contains`].
#[must_use]
pub fn replay(
    steps: &[ReplayStep],
    profile: &TerminalProfile,
    emulator: &mut dyn TerminalEmulator,
) -> ReplayReport {
    replay_with_mode(steps, profile, emulator, MatchMode::Contains)
}

/// Feed each action into `emulator` and compare using `mode`.
#[must_use]
pub fn replay_with_mode(
    steps: &[ReplayStep],
    profile: &TerminalProfile,
    emulator: &mut dyn TerminalEmulator,
    mode: MatchMode,
) -> ReplayReport {
    let mut report = ReplayReport {
        steps: steps.len(),
        divergences: Vec::new(),
    };
    for step in steps {
        for encoded in encode(&step.action, profile) {
            if !encoded.bytes.is_empty() {
                emulator.feed(&encoded.bytes);
            }
        }
        if mode == MatchMode::Skip {
            continue;
        }
        if let Some(expected) = &step.expected {
            let actual = emulator.screen_text();
            let matched = match mode {
                MatchMode::Contains => actual.contains(expected),
                MatchMode::Exact => actual.trim_end() == expected.trim_end(),
                MatchMode::Skip => true,
            };
            if !matched {
                report.divergences.push(Divergence {
                    at_ms: step.at_ms,
                    expected: expected.clone(),
                    actual,
                });
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::action::ActionPlan;
    use crate::term::Vt100Emulator;

    #[test]
    fn replay_reaches_expected_screen_state() {
        let profile = TerminalProfile::vt100(24, 80);
        let steps = vec![
            ReplayStep {
                at_ms: 0,
                action: ActionPlan::text("hello"),
                expected: Some("hello".into()),
            },
            ReplayStep {
                at_ms: 10,
                action: ActionPlan::command("echo hi"),
                expected: Some("hi".into()),
            },
        ];
        let mut emulator = Vt100Emulator::new(profile.clone());
        let report = replay(&steps, &profile, &mut emulator);
        assert!(report.passed(), "{report:?}");
    }

    #[test]
    fn replay_reports_the_divergent_frame() {
        let profile = TerminalProfile::vt100(24, 80);
        let steps = vec![ReplayStep {
            at_ms: 5,
            action: ActionPlan::text("hello"),
            expected: Some("goodbye".into()),
        }];
        let mut emulator = Vt100Emulator::new(profile.clone());
        let report = replay(&steps, &profile, &mut emulator);
        assert!(!report.passed());
        assert_eq!(report.divergences[0].expected, "goodbye");
        assert!(report.divergences[0].actual.contains("hello"));
    }

    #[test]
    fn exact_mode_rejects_extra_output_and_skip_mode_ignores_expectations() {
        let profile = TerminalProfile::vt100(10, 40);
        let steps = vec![
            ReplayStep {
                at_ms: 0,
                action: ActionPlan::text("hi"),
                expected: Some("hi".into()),
            },
            ReplayStep {
                at_ms: 1,
                action: ActionPlan::text(" there"),
                expected: Some("hi".into()),
            },
        ];
        let mut emulator = Vt100Emulator::new(profile.clone());
        let exact = replay_with_mode(&steps, &profile, &mut emulator, MatchMode::Exact);
        assert!(!exact.passed(), "extra output must fail exact mode");

        let steps = vec![ReplayStep {
            at_ms: 0,
            action: ActionPlan::text("hi"),
            expected: Some("never seen".into()),
        }];
        let mut emulator = Vt100Emulator::new(profile.clone());
        let skipped = replay_with_mode(&steps, &profile, &mut emulator, MatchMode::Skip);
        assert!(skipped.passed());
    }
}
