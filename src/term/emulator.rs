//! A minimal emulator seam so a second renderer can be added without changing
//! the observation loop.
//!
//! The trait exposes the least common denominator: bytes in, screen text and
//! capability profile out. Styled cells, cursor state, and scrollback stay on
//! the concrete type until a consumer needs them through the seam.

use crate::contract::TerminalProfile;
use crate::term::TerminalObserver;

pub trait TerminalEmulator {
    fn feed(&mut self, bytes: &[u8]);
    fn resize(&mut self, rows: u16, cols: u16);
    fn screen_text(&self) -> String;
    fn profile(&self) -> &TerminalProfile;
}

/// `vt100`-backed emulator. The profile declares what this renderer supports;
/// it does not probe the far end.
pub struct Vt100Emulator {
    observer: TerminalObserver,
    profile: TerminalProfile,
}

impl Vt100Emulator {
    #[must_use]
    pub fn new(profile: TerminalProfile) -> Self {
        let observer = TerminalObserver::new(profile.rows, profile.cols);
        Self { observer, profile }
    }

    #[must_use]
    pub fn observer(&self) -> &TerminalObserver {
        &self.observer
    }
}

impl TerminalEmulator for Vt100Emulator {
    fn feed(&mut self, bytes: &[u8]) {
        self.observer.process(bytes);
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        self.observer.resize(rows, cols);
        self.profile.rows = rows;
        self.profile.cols = cols;
    }

    fn screen_text(&self) -> String {
        self.observer.snapshot().contents
    }

    fn profile(&self) -> &TerminalProfile {
        &self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_reach_the_screen_and_resize_updates_the_profile() {
        let mut emulator = Vt100Emulator::new(TerminalProfile::vt100(24, 80));
        emulator.feed(b"hello\r\n");
        assert!(emulator.screen_text().contains("hello"));

        emulator.resize(40, 120);
        assert_eq!(emulator.profile().rows, 40);
        assert_eq!(emulator.profile().cols, 120);
    }

    #[test]
    fn capability_presets_differ_where_it_matters() {
        let vt100 = TerminalProfile::vt100(24, 80);
        let ghostty = TerminalProfile::ghostty(24, 80);
        assert!(!vt100.bracketed_paste);
        assert!(ghostty.bracketed_paste);
        assert!(!vt100.kitty_keyboard);
        assert!(ghostty.kitty_keyboard);
    }
}
