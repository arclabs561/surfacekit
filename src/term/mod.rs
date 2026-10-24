//! Real PTY and VT100 screen observation for terminal agents.
//!
//! The layer is intentionally transport-honest: [`pty::PtyHandle`] owns a real
//! pseudo-terminal, [`terminal::TerminalObserver`] parses its byte stream, and
//! [`workspace::WorkspaceObserver`] records filesystem side effects. A rendered
//! screen is exposed both as the legacy [`types::ScreenSnapshot`] and as the
//! newer [`screen::ScreenObserver`] grid; the two are kept namespaced because
//! they model color and cell data independently.

pub mod emulator;
pub mod pty;
pub mod recording;
pub mod screen;
pub mod terminal;
pub mod types;
pub mod workspace;

pub use emulator::{TerminalEmulator, Vt100Emulator};
pub use pty::PtyHandle;
pub use recording::{RecordingPaths, TuiRecording};
pub use terminal::TerminalObserver;
pub use types::{
    safe_truncate, Action, Bug, Color, CritiqueEntry, Finding, NextAction, ScreenSnapshot,
    Severity, SpecialKey, StyledCell, StyledRow,
};
pub use workspace::{WorkspaceDiff, WorkspaceObserver};
