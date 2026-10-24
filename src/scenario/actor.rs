//! Actors drive a session; the loop records what happened.
//!
//! An actor decides from the rendered screen, not from a private script. The
//! loop owns pacing, the timeline, and termination; the actor owns intent.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::contract::TerminalProfile;
use crate::scenario::action::{encode, ActionPlan};
use crate::scenario::critique::{Critic, CritiqueBudget, CritiqueContext};
use crate::scenario::timeline::{Event, Timeline, VerificationProvenance};
use crate::term::{SessionIo, TerminalEmulator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation<'a> {
    pub screen: &'a str,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorDecision {
    Act(ActionPlan),
    Wait(Duration),
    Stop,
}

pub trait Actor {
    fn next(&mut self, observation: Observation<'_>) -> ActorDecision;
}

/// Emit plans in order, then stop.
pub struct ScriptedActor {
    plans: VecDeque<ActionPlan>,
}

impl ScriptedActor {
    #[must_use]
    pub fn new(plans: Vec<ActionPlan>) -> Self {
        Self {
            plans: plans.into(),
        }
    }
}

impl Actor for ScriptedActor {
    fn next(&mut self, _observation: Observation<'_>) -> ActorDecision {
        self.plans
            .pop_front()
            .map_or(ActorDecision::Stop, ActorDecision::Act)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectStep {
    pub wait_for: Option<String>,
    pub action: Option<ActionPlan>,
}

impl ExpectStep {
    #[must_use]
    pub fn act_on(wait_for: impl Into<String>, action: ActionPlan) -> Self {
        Self {
            wait_for: Some(wait_for.into()),
            action: Some(action),
        }
    }

    #[must_use]
    pub fn wait_for(wait_for: impl Into<String>) -> Self {
        Self {
            wait_for: Some(wait_for.into()),
            action: None,
        }
    }
}

/// Wait for a screen pattern before acting, then stop.
pub struct ExpectActor {
    steps: VecDeque<ExpectStep>,
}

impl ExpectActor {
    #[must_use]
    pub fn new(steps: Vec<ExpectStep>) -> Self {
        Self {
            steps: steps.into(),
        }
    }
}

impl Actor for ExpectActor {
    fn next(&mut self, observation: Observation<'_>) -> ActorDecision {
        let Some(front) = self.steps.front() else {
            return ActorDecision::Stop;
        };
        if let Some(pattern) = &front.wait_for {
            if !observation.screen.contains(pattern) {
                return ActorDecision::Wait(Duration::from_millis(25));
            }
        }
        let step = self.steps.pop_front().expect("front existed");
        step.action
            .map_or(ActorDecision::Wait(Duration::ZERO), ActorDecision::Act)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopConfig {
    pub timeout: Duration,
    pub poll: Duration,
    pub max_steps: usize,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            poll: Duration::from_millis(10),
            max_steps: 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopReport {
    pub timeline: Timeline,
    pub steps: usize,
    pub timed_out: bool,
    pub dead: bool,
    pub error: Option<String>,
}

/// Run an actor against a session until it stops, times out, or the transport
/// closes. Output is drained before every decision so the actor sees the
/// effect of its last action.
#[must_use]
pub fn run_actor(
    actor: &mut dyn Actor,
    io: &mut dyn SessionIo,
    emulator: &mut dyn TerminalEmulator,
    profile: &TerminalProfile,
    config: &LoopConfig,
) -> LoopReport {
    run_actor_inner(actor, io, emulator, profile, config, None)
}

/// Run the actor loop and let a live critic answer after each action. The
/// loop owns the latency budget: it measures the call and records an
/// over-budget critique instead of dropping it. It cannot preempt a
/// synchronous critic.
pub fn run_actor_with_critic(
    actor: &mut dyn Actor,
    io: &mut dyn SessionIo,
    emulator: &mut dyn TerminalEmulator,
    profile: &TerminalProfile,
    config: &LoopConfig,
    critic: &mut dyn Critic,
    budget: CritiqueBudget,
) -> LoopReport {
    run_actor_inner(actor, io, emulator, profile, config, Some((critic, budget)))
}

fn run_actor_inner(
    actor: &mut dyn Actor,
    io: &mut dyn SessionIo,
    emulator: &mut dyn TerminalEmulator,
    profile: &TerminalProfile,
    config: &LoopConfig,
    mut critic: Option<(&mut dyn Critic, CritiqueBudget)>,
) -> LoopReport {
    let start = Instant::now();
    let mut timeline = Timeline::new();
    let mut steps = 0;
    let mut timed_out = false;
    let mut dead = false;
    let mut error = None;

    'outer: loop {
        let mut buffer = [0u8; 4096];
        loop {
            match io.read_available(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    emulator.feed(&buffer[..n]);
                    timeline.push(Event::Output {
                        at_ms: elapsed(start),
                        bytes: buffer[..n].to_vec(),
                    });
                }
                Err(_) => break,
            }
        }

        if start.elapsed() >= config.timeout {
            timed_out = true;
            break;
        }
        if io.is_dead() {
            dead = true;
            break;
        }

        let screen = emulator.screen_text();
        let observation = Observation {
            screen: &screen,
            elapsed_ms: elapsed(start),
        };
        match actor.next(observation) {
            ActorDecision::Act(plan) => {
                for encoded in encode(&plan, profile) {
                    if !encoded.bytes.is_empty() {
                        if let Err(write_error) = io.write_bytes(&encoded.bytes) {
                            error = Some(write_error.to_string());
                            dead = true;
                            break 'outer;
                        }
                    }
                    if let Some(delay) = encoded.delay.filter(|delay| !delay.is_zero()) {
                        std::thread::sleep(delay.min(config.poll));
                    }
                }
                if let Some((critic, budget)) = critic.as_mut() {
                    let started = Instant::now();
                    let verdict = critic.critique(CritiqueContext {
                        screen: &screen,
                        action: Some(&plan),
                        elapsed_ms: elapsed(start),
                    });
                    if let Some(verdict) = verdict {
                        let latency_ms = started.elapsed().as_millis() as u64;
                        timeline.push(Event::Critique {
                            at_ms: elapsed(start),
                            verdict,
                            provenance: VerificationProvenance::Judge,
                            latency_ms: Some(latency_ms),
                            within_budget: Some(latency_ms <= budget.max_latency_ms),
                        });
                    }
                }
                timeline.push(Event::Action {
                    at_ms: elapsed(start),
                    plan,
                });
                steps += 1;
                if steps >= config.max_steps {
                    break;
                }
            }
            ActorDecision::Wait(duration) => {
                std::thread::sleep(duration.min(config.poll).max(Duration::from_millis(1)));
            }
            ActorDecision::Stop => break,
        }
    }

    timeline.push(Event::Exit {
        at_ms: elapsed(start),
        clean: !timed_out && !dead,
    });
    LoopReport {
        timeline,
        steps,
        timed_out,
        dead,
        error,
    }
}

fn elapsed(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::action::{Resolution, Step};
    use crate::term::{CommandPty, Vt100Emulator};
    use std::io;
    use std::process::Command;

    struct MockIo {
        scripted: VecDeque<Vec<u8>>,
        written: Vec<u8>,
        dead: bool,
    }

    impl SessionIo for MockIo {
        fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.written.extend_from_slice(bytes);
            Ok(())
        }

        fn read_available(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if let Some(chunk) = self.scripted.pop_front() {
                let n = chunk.len().min(buffer.len());
                buffer[..n].copy_from_slice(&chunk[..n]);
                Ok(n)
            } else {
                Ok(0)
            }
        }

        fn resize(&mut self, _rows: u16, _cols: u16) -> io::Result<()> {
            Ok(())
        }

        fn is_dead(&self) -> bool {
            self.dead
        }
    }

    #[test]
    fn scripted_actor_records_actions_and_output() {
        let mut io = MockIo {
            scripted: VecDeque::from([b"READY\r\n".to_vec(), b"GOT:hi\r\n".to_vec()]),
            written: Vec::new(),
            dead: false,
        };
        let mut emulator = Vt100Emulator::new(TerminalProfile::vt100(10, 40));
        let mut actor = ScriptedActor::new(vec![ActionPlan::text("hi")]);
        let config = LoopConfig {
            timeout: Duration::from_secs(1),
            poll: Duration::from_millis(1),
            max_steps: 4,
        };
        let report = run_actor(
            &mut actor,
            &mut io,
            &mut emulator,
            &TerminalProfile::vt100(10, 40),
            &config,
        );
        assert_eq!(report.steps, 1);
        assert!(io.written.windows(2).any(|window| window == b"hi"));
        assert!(report
            .timeline
            .events()
            .iter()
            .any(|event| matches!(event, Event::Action { .. })));
        assert!(report
            .timeline
            .events()
            .iter()
            .any(|event| matches!(event, Event::Exit { clean: true, .. })));
    }

    struct FixedCritic {
        calls: usize,
        verdict: String,
    }

    impl Critic for FixedCritic {
        fn critique(&mut self, context: CritiqueContext<'_>) -> Option<String> {
            self.calls += 1;
            assert!(context.action.is_some(), "critic should see the plan");
            Some(self.verdict.clone())
        }
    }

    struct SlowCritic {
        delay: Duration,
    }

    impl Critic for SlowCritic {
        fn critique(&mut self, _context: CritiqueContext<'_>) -> Option<String> {
            std::thread::sleep(self.delay);
            Some("slow".to_string())
        }
    }

    #[test]
    fn critic_records_a_bounded_critique() {
        let mut io = MockIo {
            scripted: VecDeque::from([b"READY\r\n".to_vec()]),
            written: Vec::new(),
            dead: false,
        };
        let mut emulator = Vt100Emulator::new(TerminalProfile::vt100(10, 40));
        let mut actor = ScriptedActor::new(vec![ActionPlan::text("hi")]);
        let config = LoopConfig {
            timeout: Duration::from_secs(1),
            poll: Duration::from_millis(1),
            max_steps: 4,
        };
        let mut critic = FixedCritic {
            calls: 0,
            verdict: "looks fine".to_string(),
        };
        let report = run_actor_with_critic(
            &mut actor,
            &mut io,
            &mut emulator,
            &TerminalProfile::vt100(10, 40),
            &config,
            &mut critic,
            CritiqueBudget {
                max_latency_ms: 1_000,
            },
        );
        assert_eq!(critic.calls, 1);
        let critique = report
            .timeline
            .events()
            .iter()
            .find_map(|event| match event {
                Event::Critique {
                    verdict,
                    latency_ms,
                    within_budget,
                    ..
                } => Some((verdict.clone(), *latency_ms, *within_budget)),
                _ => None,
            })
            .expect("critique recorded");
        assert_eq!(critique.0, "looks fine");
        assert!(critique.1.is_some(), "latency measured");
        assert_eq!(critique.2, Some(true));
    }

    #[test]
    fn over_budget_critique_is_recorded_not_dropped() {
        let mut io = MockIo {
            scripted: VecDeque::from([b"READY\r\n".to_vec()]),
            written: Vec::new(),
            dead: false,
        };
        let mut emulator = Vt100Emulator::new(TerminalProfile::vt100(10, 40));
        let mut actor = ScriptedActor::new(vec![ActionPlan::text("hi")]);
        let config = LoopConfig {
            timeout: Duration::from_secs(1),
            poll: Duration::from_millis(1),
            max_steps: 4,
        };
        let mut critic = SlowCritic {
            delay: Duration::from_millis(20),
        };
        let report = run_actor_with_critic(
            &mut actor,
            &mut io,
            &mut emulator,
            &TerminalProfile::vt100(10, 40),
            &config,
            &mut critic,
            CritiqueBudget { max_latency_ms: 1 },
        );
        let mut found = false;
        for event in report.timeline.events() {
            if let Event::Critique {
                verdict,
                within_budget,
                ..
            } = event
            {
                assert_eq!(verdict, "slow");
                assert_eq!(*within_budget, Some(false));
                found = true;
            }
        }
        assert!(found, "over-budget critique retained");
    }

    #[test]
    fn actor_drives_a_real_pty_and_records_a_timeline() {
        let mut command = Command::new("sh");
        command.arg("-c").arg(
            "printf 'READY\\n'; while IFS= read -r line; do printf 'GOT:%s\\n' \"$line\"; done",
        );
        let Ok(mut io) = CommandPty::spawn(command, 10, 60) else {
            // Some sandboxes deny pty allocation; the unit loop above still
            // covers the logic.
            return;
        };
        let profile = TerminalProfile::vt100(10, 60);
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
        let config = LoopConfig {
            timeout: Duration::from_secs(5),
            poll: Duration::from_millis(10),
            max_steps: 4,
        };
        let report = run_actor(&mut actor, &mut io, &mut emulator, &profile, &config);
        assert!(!report.timed_out, "{:?}", report.timeline);
        assert!(
            emulator.screen_text().contains("GOT:hello"),
            "screen: {:?}",
            emulator.screen_text()
        );
    }
}
