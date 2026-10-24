//! Append-only, monotonic event record for one session.
//!
//! The shape follows asciicast v3's discipline: input, output, resize, marker,
//! and exit are distinct event kinds with monotonic timestamps. A semantic
//! action code is added because a keystroke-only record cannot say what the
//! actor was trying to do.

use crate::scenario::action::{ActionPlan, Key, KeyAction, Modifiers};
use serde::{Deserialize, Serialize};

/// Who produced a verification verdict. A program check outranks a judge; a
/// judge outranks an unlabelled opinion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationProvenance {
    Program,
    Judge,
    Human,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Event {
    /// Bytes the terminal produced.
    Output {
        at_ms: u64,
        bytes: Vec<u8>,
    },
    /// A semantic action the actor intended.
    Action {
        at_ms: u64,
        plan: ActionPlan,
    },
    /// One physical key event.
    Key {
        at_ms: u64,
        action: KeyAction,
        key: Key,
        modifiers: Modifiers,
    },
    Resize {
        at_ms: u64,
        rows: u16,
        cols: u16,
    },
    /// Annotation or regression label. Never a side channel.
    Marker {
        at_ms: u64,
        label: String,
    },
    /// Deterministic check result.
    Check {
        at_ms: u64,
        name: String,
        passed: bool,
        provenance: VerificationProvenance,
    },
    /// Judge observation. Latency fields are absent on records captured
    /// before the loop tracked a critique budget; absence means "not
    /// measured", never "zero".
    Critique {
        at_ms: u64,
        verdict: String,
        provenance: VerificationProvenance,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        latency_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        within_budget: Option<bool>,
    },
    Exit {
        at_ms: u64,
        clean: bool,
    },
}

impl Event {
    #[must_use]
    pub fn at_ms(&self) -> u64 {
        match self {
            Self::Output { at_ms, .. }
            | Self::Action { at_ms, .. }
            | Self::Key { at_ms, .. }
            | Self::Resize { at_ms, .. }
            | Self::Marker { at_ms, .. }
            | Self::Check { at_ms, .. }
            | Self::Critique { at_ms, .. }
            | Self::Exit { at_ms, .. } => *at_ms,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    events: Vec<Event>,
}

impl Timeline {
    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        self.events.last().map_or(0, Event::at_ms)
    }

    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        let mut out = String::new();
        for event in &self.events {
            out.push_str(&serde_json::to_string(event)?);
            out.push('\n');
        }
        Ok(out)
    }

    pub fn from_jsonl(input: &str) -> Result<Self, serde_json::Error> {
        let mut timeline = Self::new();
        for line in input.lines().filter(|line| !line.trim().is_empty()) {
            timeline.push(serde_json::from_str(line)?);
        }
        Ok(timeline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::action::{Resolution, Step};

    #[test]
    fn timeline_round_trips_as_jsonl_and_keeps_order() {
        let mut timeline = Timeline::new();
        timeline.push(Event::Action {
            at_ms: 0,
            plan: ActionPlan::new(
                Resolution::Line,
                vec![Step::Line {
                    text: "echo hi".into(),
                    submit: true,
                }],
            ),
        });
        timeline.push(Event::Output {
            at_ms: 20,
            bytes: b"hi\r\n".to_vec(),
        });
        timeline.push(Event::Marker {
            at_ms: 21,
            label: "regression: prompt-echo".into(),
        });

        let text = timeline.to_jsonl().unwrap();
        let back = Timeline::from_jsonl(&text).unwrap();
        assert_eq!(timeline, back);
        assert_eq!(back.duration_ms(), 21);
        assert_eq!(back.events().len(), 3);
    }
}
