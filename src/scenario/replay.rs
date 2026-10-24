//! Deterministic re-feed of a session into a fresh emulator.
//!
//! Replay is an assertion, not a video: the caller supplies the expected
//! observable state after each action, and divergence is reported with the
//! first frame that no longer matches.

use crate::contract::TerminalProfile;
use crate::scenario::action::{encode, ActionPlan};
use crate::scenario::actor::{run_actor, ExpectActor, ExpectStep, LoopConfig};
use crate::scenario::case::Case;
use crate::term::{SessionIo, TerminalEmulator};

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
    pub error: Option<String>,
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
        error: None,
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

/// Re-execute a case against a fresh session and compare its checks.
///
/// This is session replay, not encoding replay: the subject runs again and its
/// output is re-derived. A missing check is a divergence; a timeout is a
/// divergence for every check that never appeared.
#[must_use]
pub fn replay_case(
    case: &Case,
    io: &mut dyn SessionIo,
    emulator: &mut dyn TerminalEmulator,
    profile: &TerminalProfile,
    config: &LoopConfig,
) -> ReplayReport {
    let mut steps = vec![ExpectStep {
        wait_for: None,
        action: Some(case.action_plan()),
    }];
    for check in &case.checks {
        steps.push(ExpectStep::wait_for(check.contains.clone()));
    }
    let mut actor = ExpectActor::new(steps);
    let run = run_actor(&mut actor, io, emulator, profile, config);
    let actual = emulator.screen_text();
    let divergences = case
        .checks
        .iter()
        .filter(|check| !actual.contains(&check.contains))
        .map(|check| Divergence {
            at_ms: run.timeline.duration_ms(),
            expected: check.contains.clone(),
            actual: actual.clone(),
        })
        .collect();
    ReplayReport {
        steps: run.steps,
        divergences,
        error: run.error,
    }
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

    fn fixture() -> std::process::Command {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(
            "printf 'READY\\n'; while IFS= read -r line; do printf 'GOT:%s\\n' \"$line\"; done",
        );
        command
    }

    #[test]
    fn a_timeline_promotes_to_a_case_and_replays_against_a_fresh_session() {
        use crate::scenario::action::{Resolution, Step};
        use crate::scenario::actor::{run_actor, ExpectActor, ExpectStep, LoopConfig};
        use crate::scenario::case::{Case, CheckSpec};
        use crate::scenario::timeline::VerificationProvenance;
        use crate::term::{CommandPty, Vt100Emulator};
        use std::time::Duration;

        let profile = TerminalProfile::vt100(10, 60);
        let config = LoopConfig {
            timeout: Duration::from_secs(5),
            poll: Duration::from_millis(10),
            max_steps: 4,
        };
        let Ok(mut io) = CommandPty::spawn(fixture(), 10, 60) else {
            return;
        };
        let mut emulator = Vt100Emulator::new(profile.clone());
        let mut actor = ExpectActor::new(vec![
            ExpectStep::act_on(
                "READY",
                ActionPlan::new(
                    Resolution::Raw,
                    vec![Step::Raw {
                        bytes: b"hello\n".to_vec(),
                    }],
                ),
            ),
            ExpectStep::wait_for("GOT:hello"),
        ]);
        let run = run_actor(&mut actor, &mut io, &mut emulator, &profile, &config);
        assert!(
            emulator.screen_text().contains("GOT:hello"),
            "{:?}",
            run.timeline
        );

        let case = Case::from_timeline(
            "fixture-echo",
            &run.timeline,
            vec![CheckSpec {
                name: "echo visible".into(),
                contains: "GOT:hello".into(),
                provenance: VerificationProvenance::Program,
            }],
        );

        let Ok(mut io) = CommandPty::spawn(fixture(), 10, 60) else {
            return;
        };
        let mut emulator = Vt100Emulator::new(profile.clone());
        let report = replay_case(&case, &mut io, &mut emulator, &profile, &config);
        assert!(report.passed(), "{report:?}");

        let mut broken = case.clone();
        broken.checks[0].contains = "GOT:goodbye".into();
        let Ok(mut io) = CommandPty::spawn(fixture(), 10, 60) else {
            return;
        };
        let mut emulator = Vt100Emulator::new(profile.clone());
        let short = LoopConfig {
            timeout: Duration::from_secs(1),
            ..config
        };
        let report = replay_case(&broken, &mut io, &mut emulator, &profile, &short);
        assert!(!report.passed());
        assert_eq!(report.divergences[0].expected, "GOT:goodbye");
    }
}
