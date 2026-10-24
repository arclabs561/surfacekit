//! Action resolutions and profile-aware encoding.
//!
//! The same intent has different evidence at different resolutions. A plan
//! says "finish the task"; a physical key says "Shift+Enter, 40ms after the
//! last character". The timeline records the intent, the emitted bytes, and
//! the effect separately, so a plan can be replayed coarsely while a regression
//! case keeps the keystrokes that actually broke it.

use crate::contract::TerminalProfile;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// How much of the input path an action exercises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// A batch of steps with one intent.
    Plan,
    /// One logical operation, such as a command.
    Command,
    /// A line of text plus an optional submit.
    Line,
    /// Typed text, never bracketed.
    Text,
    /// A paste payload, bracketed when the profile supports it.
    Paste,
    /// One key with modifiers.
    KeyChord,
    /// One physical key event with timing.
    PhysicalKey,
    /// A deliberate pause.
    Wait,
    /// Raw bytes, bypassing encoding.
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Modifiers {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    #[must_use]
    pub const fn ctrl() -> Self {
        Self {
            ctrl: true,
            ..Self::none()
        }
    }

    #[must_use]
    pub const fn alt() -> Self {
        Self {
            alt: true,
            ..Self::none()
        }
    }

    #[must_use]
    pub const fn shift() -> Self {
        Self {
            shift: true,
            ..Self::none()
        }
    }

    /// Kitty keyboard protocol modifier parameter: 1 + bitmask.
    #[must_use]
    pub const fn kitty_param(self) -> u8 {
        1 + (self.shift as u8)
            + ((self.alt as u8) << 1)
            + ((self.ctrl as u8) << 2)
            + ((self.meta as u8) << 3)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    Press,
    Release,
    Repeat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    Plan {
        resolution: Resolution,
        steps: Vec<Step>,
    },
    Command {
        text: String,
    },
    Line {
        text: String,
        submit: bool,
    },
    Text {
        text: String,
    },
    Paste {
        text: String,
    },
    KeyChord {
        modifiers: Modifiers,
        key: Key,
    },
    Key {
        action: KeyAction,
        key: Key,
        delay_ms: Option<u64>,
    },
    Wait {
        ms: u64,
    },
    Raw {
        bytes: Vec<u8>,
    },
}

impl Step {
    #[must_use]
    pub fn resolution(&self) -> Resolution {
        match self {
            Self::Plan { .. } => Resolution::Plan,
            Self::Command { .. } => Resolution::Command,
            Self::Line { .. } => Resolution::Line,
            Self::Text { .. } => Resolution::Text,
            Self::Paste { .. } => Resolution::Paste,
            Self::KeyChord { .. } => Resolution::KeyChord,
            Self::Key { .. } => Resolution::PhysicalKey,
            Self::Wait { .. } => Resolution::Wait,
            Self::Raw { .. } => Resolution::Raw,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPlan {
    pub resolution: Resolution,
    pub steps: Vec<Step>,
}

impl ActionPlan {
    #[must_use]
    pub fn new(resolution: Resolution, steps: Vec<Step>) -> Self {
        Self { resolution, steps }
    }

    #[must_use]
    pub fn command(text: impl Into<String>) -> Self {
        Self::new(
            Resolution::Command,
            vec![Step::Command { text: text.into() }],
        )
    }

    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(Resolution::Text, vec![Step::Text { text: text.into() }])
    }
}

/// One encoded unit: bytes to write plus an optional pause before the next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedAction {
    pub bytes: Vec<u8>,
    pub delay: Option<Duration>,
}

/// Encode a plan for a declared terminal profile.
///
/// Legacy terminals get legacy sequences. Kitty-aware profiles get `CSI u` for
/// character chords; special keys still use legacy sequences in this slice.
#[must_use]
pub fn encode(plan: &ActionPlan, profile: &TerminalProfile) -> Vec<EncodedAction> {
    let mut out = Vec::new();
    for step in &plan.steps {
        encode_step(step, profile, &mut out);
    }
    out
}

fn encode_step(step: &Step, profile: &TerminalProfile, out: &mut Vec<EncodedAction>) {
    match step {
        Step::Plan { steps, .. } => {
            for child in steps {
                encode_step(child, profile, out);
            }
        }
        Step::Command { text } => out.push(EncodedAction {
            bytes: format!("{text}\r").into_bytes(),
            delay: None,
        }),
        Step::Line { text, submit } => {
            let mut bytes = text.clone().into_bytes();
            if *submit {
                bytes.push(b'\r');
            }
            out.push(EncodedAction { bytes, delay: None });
        }
        Step::Text { text } => out.push(EncodedAction {
            bytes: text.clone().into_bytes(),
            delay: None,
        }),
        Step::Paste { text } => out.push(EncodedAction {
            bytes: if profile.bracketed_paste {
                format!("\x1b[200~{text}\x1b[201~").into_bytes()
            } else {
                text.clone().into_bytes()
            },
            delay: None,
        }),
        Step::KeyChord { modifiers, key } => out.push(EncodedAction {
            bytes: encode_key(*key, *modifiers, true, profile),
            delay: None,
        }),
        Step::Wait { ms } => out.push(EncodedAction {
            bytes: Vec::new(),
            delay: Some(Duration::from_millis(*ms)),
        }),
        Step::Raw { bytes } => out.push(EncodedAction {
            bytes: bytes.clone(),
            delay: None,
        }),
        Step::Key {
            action,
            key,
            delay_ms,
        } => {
            // Legacy terminals have no release/repeat event. Only emit a press
            // unless the profile advertises kitty keyboard support.
            let emit = *action == KeyAction::Press || profile.kitty_keyboard;
            if emit {
                out.push(EncodedAction {
                    bytes: encode_key(*key, Modifiers::none(), false, profile),
                    delay: delay_ms.map(Duration::from_millis),
                });
            }
        }
    }
}

fn encode_key(key: Key, modifiers: Modifiers, chord: bool, profile: &TerminalProfile) -> Vec<u8> {
    if let Key::Char(c) = key {
        if chord && profile.kitty_keyboard && (modifiers.ctrl || modifiers.alt || modifiers.shift) {
            return format!("\x1b[{};{}u", u32::from(c), modifiers.kitty_param()).into_bytes();
        }
        if modifiers.ctrl {
            if let Some(byte) = control_byte(c) {
                return vec![byte];
            }
        }
        let mut bytes = Vec::new();
        if modifiers.alt {
            bytes.push(0x1b);
        }
        let mut buffer = [0u8; 4];
        bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
        return bytes;
    }

    let sequence = match key {
        Key::Enter => "\r",
        Key::Tab => "\t",
        Key::Escape => "\x1b",
        Key::Backspace => "\x7f",
        Key::Delete => "\x1b[3~",
        Key::Up => "\x1b[A",
        Key::Down => "\x1b[B",
        Key::Right => "\x1b[C",
        Key::Left => "\x1b[D",
        Key::Home => "\x1b[H",
        Key::End => "\x1b[F",
        Key::Char(_) => unreachable!(),
    };
    let mut bytes = Vec::new();
    if modifiers.alt {
        bytes.push(0x1b);
    }
    bytes.extend_from_slice(sequence.as_bytes());
    bytes
}

fn control_byte(c: char) -> Option<u8> {
    let lower = c.to_ascii_lowercase();
    if lower.is_ascii_lowercase() {
        Some((lower as u8) - b'a' + 1)
    } else if c == '[' {
        Some(0x1b)
    } else if c == '\\' {
        Some(0x1c)
    } else if c == ']' {
        Some(0x1d)
    } else if c == '^' {
        Some(0x1e)
    } else if c == '_' {
        Some(0x1f)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::TerminalProfile;

    fn bytes(plan: &ActionPlan, profile: &TerminalProfile) -> Vec<u8> {
        encode(plan, profile)
            .into_iter()
            .flat_map(|action| action.bytes)
            .collect()
    }

    #[test]
    fn a_command_submits_with_enter() {
        let plan = ActionPlan::command("ls");
        assert_eq!(bytes(&plan, &TerminalProfile::vt100(24, 80)), b"ls\r");
    }

    #[test]
    fn typed_text_never_gets_paste_markers() {
        let plan = ActionPlan::text("hi");
        assert_eq!(bytes(&plan, &TerminalProfile::vt100(24, 80)), b"hi");
        assert_eq!(bytes(&plan, &TerminalProfile::xterm(24, 80)), b"hi");
    }

    #[test]
    fn paste_is_bracketed_only_when_the_profile_supports_it() {
        let plan = ActionPlan::new(Resolution::Paste, vec![Step::Paste { text: "hi".into() }]);
        assert_eq!(bytes(&plan, &TerminalProfile::vt100(24, 80)), b"hi");
        assert_eq!(
            bytes(&plan, &TerminalProfile::xterm(24, 80)),
            b"\x1b[200~hi\x1b[201~".to_vec()
        );
    }

    #[test]
    fn control_and_alt_chords_encode_to_expected_bytes() {
        let ctrl_c = ActionPlan::new(
            Resolution::KeyChord,
            vec![Step::KeyChord {
                modifiers: Modifiers::ctrl(),
                key: Key::Char('c'),
            }],
        );
        assert_eq!(bytes(&ctrl_c, &TerminalProfile::vt100(24, 80)), vec![0x03]);

        let alt_x = ActionPlan::new(
            Resolution::KeyChord,
            vec![Step::KeyChord {
                modifiers: Modifiers::alt(),
                key: Key::Char('x'),
            }],
        );
        assert_eq!(
            bytes(&alt_x, &TerminalProfile::vt100(24, 80)),
            b"\x1bx".to_vec()
        );
    }

    #[test]
    fn kitty_aware_profiles_use_csi_u_for_character_chords() {
        let plan = ActionPlan::new(
            Resolution::KeyChord,
            vec![Step::KeyChord {
                modifiers: Modifiers::ctrl(),
                key: Key::Char('a'),
            }],
        );
        assert_eq!(
            bytes(&plan, &TerminalProfile::ghostty(24, 80)),
            b"\x1b[97;5u".to_vec()
        );
    }

    #[test]
    fn nested_plans_flatten_in_order() {
        let plan = ActionPlan::new(
            Resolution::Plan,
            vec![
                Step::Plan {
                    resolution: Resolution::Line,
                    steps: vec![
                        Step::Line {
                            text: "echo hi".into(),
                            submit: true,
                        },
                        Step::Key {
                            action: KeyAction::Press,
                            key: Key::Up,
                            delay_ms: Some(5),
                        },
                    ],
                },
                Step::Text { text: "x".into() },
            ],
        );
        let encoded = encode(&plan, &TerminalProfile::vt100(24, 80));
        assert_eq!(encoded.len(), 3);
        assert_eq!(encoded[0].bytes, b"echo hi\r");
        assert_eq!(encoded[1].bytes, b"\x1b[A");
        assert_eq!(encoded[1].delay, Some(Duration::from_millis(5)));
        assert_eq!(encoded[2].bytes, b"x");
    }

    #[test]
    fn legacy_profiles_do_not_emit_release_events() {
        let plan = ActionPlan::new(
            Resolution::PhysicalKey,
            vec![Step::Key {
                action: KeyAction::Release,
                key: Key::Char('a'),
                delay_ms: None,
            }],
        );
        assert!(encode(&plan, &TerminalProfile::vt100(24, 80)).is_empty());
    }

    #[test]
    fn raw_bytes_bypass_encoding() {
        let plan = ActionPlan::new(
            Resolution::Raw,
            vec![Step::Raw {
                bytes: b"\x1b[Z".to_vec(),
            }],
        );
        assert_eq!(bytes(&plan, &TerminalProfile::vt100(24, 80)), b"\x1b[Z");
    }
}
