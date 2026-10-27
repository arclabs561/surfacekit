//! A critic's running notebook: dated, append-only notes it can revise.
//!
//! A stateless critic answers each action from scratch, so it cannot notice
//! that the third screen contradicts what the first one promised. A noting
//! critic reads its own earlier notes before it writes the next one. Notes
//! are never edited in place: a revision is a new note whose `revises` names
//! the earlier one, so the record keeps what the critic believed and when.
//!
//! This module is part of the always-available contract layer because the
//! notes travel in a [`crate::SurfacePacket`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Longest note text kept, in characters. Longer drafts are cut on a
/// character boundary; notes are meant to be short.
pub const MAX_NOTE_CHARS: usize = 400;

/// What a note records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    /// What the critic saw.
    Observation,
    /// What it expected against what it experienced.
    ExpectationGap,
    /// Something that slowed or confused it.
    Friction,
    /// Something that worked better than expected.
    Delight,
    /// Something to find out on a later step.
    Question,
}

/// A note before the notebook dates and numbers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDraft {
    pub kind: NoteKind,
    pub text: String,
    /// The earlier note this one corrects or supersedes.
    pub revises: Option<u32>,
}

impl NoteDraft {
    #[must_use]
    pub fn new(kind: NoteKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            revises: None,
        }
    }

    #[must_use]
    pub fn revising(mut self, id: u32) -> Self {
        self.revises = Some(id);
        self
    }
}

/// One recorded note. Latency fields are absent when the producer did not
/// measure the critic call; absence means "not measured", never "zero".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: u32,
    pub at_ms: u64,
    /// The actor step the note follows, when the producer counts steps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<u32>,
    pub kind: NoteKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revises: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within_budget: Option<bool>,
}

/// The append-only note record for one session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Notebook {
    notes: Vec<Note>,
}

impl Notebook {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Date, number, and record a draft; returns the new note's id.
    ///
    /// A `revises` that names no earlier note is cleared rather than kept:
    /// the note is still recorded, but it does not hide anything.
    pub fn append(&mut self, draft: NoteDraft, at_ms: u64, step: Option<u32>) -> u32 {
        let id = self.notes.last().map_or(1, |note| note.id + 1);
        let revises = draft.revises.filter(|target| self.contains(*target));
        self.notes.push(Note {
            id,
            at_ms,
            step,
            kind: draft.kind,
            text: truncate(&draft.text, MAX_NOTE_CHARS),
            revises,
            latency_ms: None,
            within_budget: None,
        });
        id
    }

    /// Record the critic call's measured latency on notes `from..` (the
    /// notes that one call produced).
    pub fn set_latency(&mut self, from_id: u32, latency_ms: u64, within_budget: bool) {
        for note in self.notes.iter_mut().filter(|note| note.id >= from_id) {
            note.latency_ms = Some(latency_ms);
            note.within_budget = Some(within_budget);
        }
    }

    #[must_use]
    pub fn contains(&self, id: u32) -> bool {
        self.notes.iter().any(|note| note.id == id)
    }

    /// Every note, in the order written.
    #[must_use]
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.notes.len()
    }

    /// Notes no later note revises: what the critic currently believes.
    #[must_use]
    pub fn current(&self) -> Vec<&Note> {
        let superseded: BTreeSet<u32> = self.notes.iter().filter_map(|n| n.revises).collect();
        self.notes
            .iter()
            .filter(|note| !superseded.contains(&note.id))
            .collect()
    }

    /// The current notes as prompt text, most recent last, at most `limit`
    /// of them. Ids are shown so a critic can name the note it revises.
    #[must_use]
    pub fn render(&self, limit: usize) -> String {
        let current = self.current();
        let skip = current.len().saturating_sub(limit);
        let mut out = String::new();
        if skip > 0 {
            out.push_str(&format!("({skip} earlier notes omitted)\n"));
        }
        for note in &current[skip..] {
            let kind = serde_json::to_value(note.kind).unwrap_or_default();
            out.push_str(&format!(
                "#{} {}{}: {}\n",
                note.id,
                kind.as_str().unwrap_or("note"),
                note.step
                    .map(|s| format!(" (step {s})"))
                    .unwrap_or_default(),
                note.text
            ));
        }
        out
    }

    /// Structural rules for a recorded note list: ids strictly increase and
    /// a revision names an earlier note.
    pub fn validate(notes: &[Note]) -> Result<(), &'static str> {
        let mut seen = BTreeSet::new();
        let mut last = 0;
        for note in notes {
            if note.id <= last {
                return Err("note ids must strictly increase");
            }
            if let Some(target) = note.revises {
                if !seen.contains(&target) {
                    return Err("a note revises an id that is not earlier in the notebook");
                }
            }
            seen.insert(note.id);
            last = note.id;
        }
        Ok(())
    }
}

impl From<Vec<Note>> for Notebook {
    fn from(notes: Vec<Note>) -> Self {
        Self { notes }
    }
}

fn truncate(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_numbered_dated_and_kept_in_order() {
        let mut book = Notebook::new();
        let a = book.append(
            NoteDraft::new(NoteKind::Observation, "home loads"),
            10,
            Some(1),
        );
        let b = book.append(
            NoteDraft::new(NoteKind::Friction, "filters hidden"),
            20,
            Some(2),
        );
        assert_eq!((a, b), (1, 2));
        assert_eq!(book.notes()[1].at_ms, 20);
        assert_eq!(book.notes()[1].step, Some(2));
    }

    #[test]
    fn a_revision_supersedes_without_erasing() {
        let mut book = Notebook::new();
        let first = book.append(NoteDraft::new(NoteKind::ExpectationGap, "no sort"), 1, None);
        book.append(
            NoteDraft::new(NoteKind::Delight, "sort exists").revising(first),
            2,
            None,
        );
        assert_eq!(book.len(), 2, "the revised note is still recorded");
        let current: Vec<u32> = book.current().iter().map(|n| n.id).collect();
        assert_eq!(current, vec![2]);
        assert!(book.render(10).contains("#2 delight"));
        assert!(!book.render(10).contains("#1"));
    }

    #[test]
    fn a_revision_of_an_unknown_note_is_kept_unlinked() {
        let mut book = Notebook::new();
        book.append(
            NoteDraft::new(NoteKind::Question, "why?").revising(9),
            1,
            None,
        );
        assert_eq!(book.notes()[0].revises, None);
        assert_eq!(book.current().len(), 1);
    }

    #[test]
    fn render_keeps_the_most_recent_notes() {
        let mut book = Notebook::new();
        for i in 0..5 {
            book.append(
                NoteDraft::new(NoteKind::Observation, format!("n{i}")),
                i,
                None,
            );
        }
        let text = book.render(2);
        assert!(text.starts_with("(3 earlier notes omitted)"));
        assert!(text.contains("n4") && text.contains("n3") && !text.contains("n2"));
    }

    #[test]
    fn long_text_is_cut_on_a_character_boundary() {
        let mut book = Notebook::new();
        let long = "あ".repeat(MAX_NOTE_CHARS + 10);
        book.append(NoteDraft::new(NoteKind::Observation, long), 0, None);
        assert_eq!(book.notes()[0].text.chars().count(), MAX_NOTE_CHARS);
        assert!(book.notes()[0].text.ends_with('…'));
    }

    #[test]
    fn latency_applies_to_the_notes_of_one_call() {
        let mut book = Notebook::new();
        book.append(NoteDraft::new(NoteKind::Observation, "a"), 0, None);
        let from = book.append(NoteDraft::new(NoteKind::Observation, "b"), 1, None);
        book.append(NoteDraft::new(NoteKind::Observation, "c"), 1, None);
        book.set_latency(from, 120, true);
        assert_eq!(book.notes()[0].latency_ms, None);
        assert_eq!(book.notes()[1].latency_ms, Some(120));
        assert_eq!(book.notes()[2].within_budget, Some(true));
    }

    #[test]
    fn validation_rejects_reordered_ids_and_forward_revisions() {
        let note = |id, revises| Note {
            id,
            at_ms: 0,
            step: None,
            kind: NoteKind::Observation,
            text: String::new(),
            revises,
            latency_ms: None,
            within_budget: None,
        };
        assert!(Notebook::validate(&[note(1, None), note(2, Some(1))]).is_ok());
        assert!(Notebook::validate(&[note(2, None), note(1, None)]).is_err());
        assert!(Notebook::validate(&[note(1, Some(2)), note(2, None)]).is_err());
    }

    #[test]
    fn notebook_serializes_as_a_plain_list() {
        let mut book = Notebook::new();
        book.append(NoteDraft::new(NoteKind::Friction, "slow"), 5, Some(1));
        let value = serde_json::to_value(&book).unwrap();
        assert_eq!(
            value,
            serde_json::json!([{"id": 1, "at_ms": 5, "step": 1, "kind": "friction", "text": "slow"}])
        );
        let back: Notebook = serde_json::from_value(value).unwrap();
        assert_eq!(back, book);
    }
}
