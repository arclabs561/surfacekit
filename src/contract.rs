//! Typed, surface-neutral contracts for evaluators.
//!
//! Rust owns the meaning of these packets for CLI and TUI fixtures. The
//! serialized form is separately admitted by `schema/surface-evaluation.schema.json`
//! so browser evaluators can exchange the same evidence without importing Rust.

use crate::notes::{Note, Notebook};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const OUTPUT_RECOVERY_ROOM: &str = "output-recovery-room";

/// Current packet revision. v2 adds environment identity and executable
/// checks, because a rendered screen alone does not prove a terminal task.
/// v3 adds the critic's running notes and caller-owned scenario claims.
pub const CURRENT_SCHEMA_VERSION: u8 = 3;

/// Revisions this crate still validates. A v2 packet stays valid as long as
/// it carries nothing v3 introduced.
pub const SUPPORTED_SCHEMA_VERSIONS: &[u8] = &[2, 3];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Cli,
    Tui,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Observation {
    BoundedSubprocess,
    ManagedPty,
    BoundedPlaywright,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Argv,
    ExitStatus,
    StdoutOrArtifact,
    RunRecord,
    PtyTranscript,
    KeyEvents,
    ResizeEvents,
    BrowserEvents,
    Screenshots,
    DomPredicates,
    /// The environment the subject ran in: image/OS/toolchain identity.
    EnvironmentIdentity,
    /// A deterministic test or predicate the task itself defines.
    ExecutableCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Claim {
    OutputVisible,
    ReferenceAddressable,
    InterruptionPreserved,
    RecoveryWithoutRegeneration,
    ActorsAttributed,
    ActorContextIsolated,
    HandoffIsNoticeNotReply,
}

const REQUIRED_CLAIMS: &[Claim] = &[
    Claim::OutputVisible,
    Claim::ReferenceAddressable,
    Claim::InterruptionPreserved,
    Claim::RecoveryWithoutRegeneration,
    Claim::ActorsAttributed,
    Claim::ActorContextIsolated,
    Claim::HandoffIsNoticeNotReply,
];

/// One observed artifact. The kind says what it proves; the payload is the
/// caller-owned body. Null payloads are treated as absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfacePacket {
    pub schema_version: u8,
    pub scenario_id: String,
    pub surface: Surface,
    pub observation: Observation,
    pub evidence_kinds: BTreeSet<EvidenceKind>,
    pub claims: BTreeSet<Claim>,
    /// Claims a caller-owned scenario tests, namespaced as `scope:claim`
    /// (for example `catalogue:goal_reached`) so none can be mistaken for,
    /// or satisfy, a preset [`Claim`]. Since schema v3.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub scenario_claims: BTreeSet<String>,
    /// Populated by the observer. A declaration-only packet may leave this
    /// empty (or omit it, as the schema allows);
    /// [`SurfacePacket::validate_complete`] then fails.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    /// The critic's running notes, in the order written. Since schema v3.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<Note>,
}

impl Surface {
    #[must_use]
    pub const fn observation(self) -> Observation {
        match self {
            Self::Cli => Observation::BoundedSubprocess,
            Self::Tui => Observation::ManagedPty,
            Self::Web => Observation::BoundedPlaywright,
        }
    }

    #[must_use]
    pub fn required_evidence(self) -> BTreeSet<EvidenceKind> {
        match self {
            Self::Cli => [
                EvidenceKind::Argv,
                EvidenceKind::ExitStatus,
                EvidenceKind::StdoutOrArtifact,
                EvidenceKind::RunRecord,
                EvidenceKind::EnvironmentIdentity,
                EvidenceKind::ExecutableCheck,
            ],
            Self::Tui => [
                EvidenceKind::PtyTranscript,
                EvidenceKind::KeyEvents,
                EvidenceKind::ResizeEvents,
                EvidenceKind::RunRecord,
                EvidenceKind::EnvironmentIdentity,
                EvidenceKind::ExecutableCheck,
            ],
            Self::Web => [
                EvidenceKind::BrowserEvents,
                EvidenceKind::Screenshots,
                EvidenceKind::DomPredicates,
                EvidenceKind::RunRecord,
                EvidenceKind::EnvironmentIdentity,
                EvidenceKind::ExecutableCheck,
            ],
        }
        .into_iter()
        .collect()
    }
}

impl SurfacePacket {
    #[must_use]
    pub fn output_recovery_room(surface: Surface) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            scenario_id: OUTPUT_RECOVERY_ROOM.to_string(),
            surface,
            observation: surface.observation(),
            evidence_kinds: surface.required_evidence(),
            claims: REQUIRED_CLAIMS.iter().copied().collect(),
            scenario_claims: BTreeSet::new(),
            evidence: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Build a packet for any scenario. The caller supplies the claims and
    /// evidence kinds; [`SurfacePacket::validate_shape`] checks the surface
    /// contract without assuming the preset scenario.
    #[must_use]
    pub fn new(
        scenario_id: impl Into<String>,
        surface: Surface,
        claims: impl IntoIterator<Item = Claim>,
        evidence_kinds: impl IntoIterator<Item = EvidenceKind>,
    ) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            scenario_id: scenario_id.into(),
            surface,
            observation: surface.observation(),
            evidence_kinds: evidence_kinds.into_iter().collect(),
            claims: claims.into_iter().collect(),
            scenario_claims: BTreeSet::new(),
            evidence: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Add claims owned by a caller-defined scenario (`scope:claim`).
    #[must_use]
    pub fn with_scenario_claims(
        mut self,
        claims: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.scenario_claims
            .extend(claims.into_iter().map(Into::into));
        self
    }

    /// Attach a critic's notebook.
    #[must_use]
    pub fn with_notes(mut self, notebook: &Notebook) -> Self {
        self.notes = notebook.notes().to_vec();
        self
    }

    /// Version, observation authority, required evidence, at least one
    /// claim, namespaced scenario claims, and a well-formed note record.
    /// Scenario-specific rules belong to [`SurfacePacket::validate`].
    pub fn validate_shape(&self) -> Result<(), &'static str> {
        if !SUPPORTED_SCHEMA_VERSIONS.contains(&self.schema_version) {
            return Err("unknown surface packet schema version");
        }
        if self.schema_version < 3 && (!self.notes.is_empty() || !self.scenario_claims.is_empty()) {
            return Err("notes and scenario claims need schema version 3");
        }
        if !self.scenario_claims.iter().all(|c| is_scenario_claim(c)) {
            return Err("a scenario claim must be namespaced as scope:claim");
        }
        Notebook::validate(&self.notes)?;
        if self.observation != self.surface.observation() {
            return Err("surface has the wrong observation authority");
        }
        if !self
            .evidence_kinds
            .is_superset(&self.surface.required_evidence())
        {
            return Err("surface packet omits required evidence");
        }
        if self.claims.is_empty() && self.scenario_claims.is_empty() {
            return Err("surface packet states no claims");
        }
        Ok(())
    }

    /// Preset validation: shape plus the shared scenario's identity and claims.
    pub fn validate(&self) -> Result<(), &'static str> {
        self.validate_shape()?;
        if self.scenario_id != OUTPUT_RECOVERY_ROOM {
            return Err("unknown shared evaluation scenario");
        }
        if !self
            .claims
            .is_superset(&REQUIRED_CLAIMS.iter().copied().collect())
        {
            return Err("surface packet omits a required scenario claim");
        }
        Ok(())
    }

    /// Shape plus payload coverage. A packet passes this only when every
    /// declared kind carries a non-null payload and nothing undeclared is
    /// attached. The declaration-only preset fails here by design.
    pub fn validate_complete(&self) -> Result<(), &'static str> {
        self.validate_shape()?;
        let mut covered = BTreeSet::new();
        for item in &self.evidence {
            if !self.evidence_kinds.contains(&item.kind) {
                return Err("evidence kind was not declared");
            }
            if item.payload.is_null() {
                return Err("evidence payload is null");
            }
            covered.insert(item.kind);
        }
        if !self.evidence_kinds.is_subset(&covered) {
            return Err("declared evidence kind has no payload");
        }
        Ok(())
    }
}

/// `scope:claim`, each part lowercase ASCII letters, digits, `_`, `.`, or
/// `-`, starting with a letter or digit. Mirrors the schema's pattern.
fn is_scenario_claim(claim: &str) -> bool {
    let part = |text: &str| {
        text.chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c))
    };
    matches!(claim.split_once(':'), Some((scope, name)) if part(scope) && part(name))
}

/// How many colors the renderer can distinguish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorDepth {
    Mono,
    Ansi16,
    Ansi256,
    TrueColor,
}

/// Declared capabilities of the terminal a session runs in.
///
/// These are declarations, not probes. A consumer that needs proof must attach
/// a capability query to the evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalProfile {
    pub rows: u16,
    pub cols: u16,
    pub color: ColorDepth,
    pub alternate_screen: bool,
    pub bracketed_paste: bool,
    pub mouse: bool,
    /// SGR encoding (1006). It is an encoding, not an enabler: mouse
    /// reporting still requires one of 1000/1002/1003.
    pub sgr_mouse: bool,
    pub focus_events: bool,
    pub hyperlinks: bool,
    pub kitty_keyboard: bool,
    pub osc_133: bool,
    pub osc_7: bool,
    pub scrollback_lines: u32,
}

impl TerminalProfile {
    #[must_use]
    pub fn vt100(rows: u16, cols: u16) -> Self {
        Self {
            rows,
            cols,
            color: ColorDepth::Ansi16,
            alternate_screen: true,
            bracketed_paste: false,
            mouse: false,
            sgr_mouse: false,
            focus_events: false,
            hyperlinks: false,
            kitty_keyboard: false,
            osc_133: false,
            osc_7: false,
            scrollback_lines: 500,
        }
    }

    #[must_use]
    pub fn xterm(rows: u16, cols: u16) -> Self {
        Self {
            rows,
            cols,
            color: ColorDepth::TrueColor,
            alternate_screen: true,
            bracketed_paste: true,
            mouse: true,
            sgr_mouse: true,
            focus_events: true,
            hyperlinks: true,
            kitty_keyboard: false,
            osc_133: true,
            osc_7: true,
            scrollback_lines: 10_000,
        }
    }

    #[must_use]
    pub fn ghostty(rows: u16, cols: u16) -> Self {
        Self {
            kitty_keyboard: true,
            ..Self::xterm(rows, cols)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::{NoteDraft, NoteKind};

    #[test]
    fn every_surface_gets_its_native_observation_and_evidence() {
        for surface in [Surface::Cli, Surface::Tui, Surface::Web] {
            assert!(SurfacePacket::output_recovery_room(surface)
                .validate()
                .is_ok());
        }
    }

    #[test]
    fn semantic_validation_rejects_missing_evidence() {
        let mut packet = SurfacePacket::output_recovery_room(Surface::Tui);
        packet.evidence_kinds.remove(&EvidenceKind::ResizeEvents);
        assert_eq!(
            packet.validate(),
            Err("surface packet omits required evidence")
        );
    }

    fn complete_packet(surface: Surface) -> SurfacePacket {
        let mut packet = SurfacePacket::output_recovery_room(surface);
        packet.evidence = packet
            .evidence_kinds
            .iter()
            .map(|kind| Evidence {
                kind: *kind,
                payload: serde_json::json!({"observed": true}),
            })
            .collect();
        packet
    }

    #[test]
    fn schema_admits_every_evidence_kind_and_version() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schema/surface-evaluation.schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            serde_json::json!(CURRENT_SCHEMA_VERSION)
        );
        let listed = schema["properties"]["evidence"]["items"]["properties"]["kind"]["enum"]
            .as_array()
            .unwrap();
        for kind in [
            EvidenceKind::Argv,
            EvidenceKind::ExitStatus,
            EvidenceKind::StdoutOrArtifact,
            EvidenceKind::RunRecord,
            EvidenceKind::PtyTranscript,
            EvidenceKind::KeyEvents,
            EvidenceKind::ResizeEvents,
            EvidenceKind::BrowserEvents,
            EvidenceKind::Screenshots,
            EvidenceKind::DomPredicates,
            EvidenceKind::EnvironmentIdentity,
            EvidenceKind::ExecutableCheck,
        ] {
            let value = serde_json::to_value(kind).unwrap();
            assert!(listed.contains(&value), "schema omits {value}");
        }
    }

    #[test]
    fn generic_packets_validate_without_the_preset_scenario() {
        let packet = SurfacePacket::new(
            "custom-scenario",
            Surface::Cli,
            [Claim::OutputVisible],
            Surface::Cli.required_evidence(),
        );
        assert!(packet.validate_shape().is_ok());
        assert_eq!(packet.validate(), Err("unknown shared evaluation scenario"));
    }

    #[test]
    fn a_packet_without_claims_is_rejected() {
        let packet = SurfacePacket::new(
            "custom-scenario",
            Surface::Cli,
            std::iter::empty::<Claim>(),
            Surface::Cli.required_evidence(),
        );
        assert_eq!(
            packet.validate_shape(),
            Err("surface packet states no claims")
        );
    }

    #[test]
    fn complete_evidence_round_trips() {
        let packet = complete_packet(Surface::Tui);
        assert!(packet.validate_complete().is_ok());
        let text = serde_json::to_string(&packet).unwrap();
        let back: SurfacePacket = serde_json::from_str(&text).unwrap();
        assert_eq!(packet, back);
    }

    #[test]
    fn declaration_only_packets_fail_completeness() {
        let packet = SurfacePacket::output_recovery_room(Surface::Tui);
        assert_eq!(
            packet.validate_complete(),
            Err("declared evidence kind has no payload")
        );
    }

    #[test]
    fn undeclared_or_null_evidence_is_rejected() {
        let mut packet = complete_packet(Surface::Tui);
        packet.evidence.push(Evidence {
            kind: EvidenceKind::Argv,
            payload: serde_json::json!({}),
        });
        assert_eq!(
            packet.validate_complete(),
            Err("evidence kind was not declared")
        );

        let mut packet = complete_packet(Surface::Tui);
        packet.evidence[0].payload = serde_json::Value::Null;
        assert_eq!(packet.validate_complete(), Err("evidence payload is null"));
    }

    fn schema() -> serde_json::Value {
        serde_json::from_str(include_str!("../schema/surface-evaluation.schema.json")).unwrap()
    }

    fn web_packet(scenario: &str) -> SurfacePacket {
        SurfacePacket::new(
            scenario,
            Surface::Web,
            std::iter::empty::<Claim>(),
            Surface::Web.required_evidence(),
        )
    }

    #[test]
    fn a_v2_packet_still_validates_but_cannot_carry_v3_fields() {
        let mut packet = complete_packet(Surface::Web);
        packet.schema_version = 2;
        assert!(packet.validate_complete().is_ok());
        let text = serde_json::to_string(&packet).unwrap();
        assert!(!text.contains("notes") && !text.contains("scenario_claims"));

        let mut book = Notebook::new();
        book.append(NoteDraft::new(NoteKind::Friction, "slow"), 0, None);
        let packet = packet.with_notes(&book);
        assert_eq!(
            packet.validate_shape(),
            Err("notes and scenario claims need schema version 3")
        );
        let mut unknown = complete_packet(Surface::Web);
        unknown.schema_version = 1;
        assert_eq!(
            unknown.validate_shape(),
            Err("unknown surface packet schema version")
        );
    }

    #[test]
    fn scenario_claims_alone_satisfy_the_claim_rule_when_namespaced() {
        let packet =
            web_packet("catalogue-explore").with_scenario_claims(["catalogue:goal_reached"]);
        assert!(packet.validate_shape().is_ok());
        assert_eq!(packet.validate(), Err("unknown shared evaluation scenario"));
        for bad in [
            "goal_reached",
            "Catalogue:goal",
            "catalogue:",
            ":goal",
            "a:b c",
        ] {
            let packet = web_packet("catalogue-explore").with_scenario_claims([bad]);
            assert_eq!(
                packet.validate_shape(),
                Err("a scenario claim must be namespaced as scope:claim"),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_scenario_claim_never_satisfies_a_preset_claim() {
        let mut packet = complete_packet(Surface::Tui);
        packet.claims.clear();
        packet = packet.with_scenario_claims(["preset:output_visible"]);
        assert!(packet.validate_shape().is_ok());
        assert_eq!(
            packet.validate(),
            Err("surface packet omits a required scenario claim")
        );
    }

    #[test]
    fn notes_travel_in_the_packet_and_must_be_well_formed() {
        let mut book = Notebook::new();
        let first = book.append(
            NoteDraft::new(NoteKind::Observation, "rows load"),
            10,
            Some(1),
        );
        book.append(
            NoteDraft::new(NoteKind::ExpectationGap, "rows repeat a series").revising(first),
            20,
            Some(2),
        );
        let packet = web_packet("catalogue-explore")
            .with_scenario_claims(["catalogue:recommendations_judged"])
            .with_notes(&book);
        assert!(packet.validate_shape().is_ok());
        let back: SurfacePacket =
            serde_json::from_str(&serde_json::to_string(&packet).unwrap()).unwrap();
        assert_eq!(back, packet);

        let mut broken = packet.clone();
        broken.notes[1].revises = Some(7);
        assert_eq!(
            broken.validate_shape(),
            Err("a note revises an id that is not earlier in the notebook")
        );
    }

    #[test]
    fn schema_lists_every_note_kind_and_admits_v3_fields() {
        let schema = schema();
        let kinds = schema["$defs"]["note"]["properties"]["kind"]["enum"]
            .as_array()
            .unwrap();
        for kind in [
            NoteKind::Observation,
            NoteKind::ExpectationGap,
            NoteKind::Friction,
            NoteKind::Delight,
            NoteKind::Question,
        ] {
            let value = serde_json::to_value(kind).unwrap();
            assert!(kinds.contains(&value), "schema omits note kind {value}");
        }
        assert_eq!(
            schema["$defs"]["note"]["properties"]["text"]["maxLength"],
            serde_json::json!(crate::notes::MAX_NOTE_CHARS)
        );
        for property in ["notes", "scenario_claims"] {
            assert!(schema["properties"][property].is_object(), "{property}");
        }
    }

    #[test]
    fn schema_examples_deserialize_and_validate_their_shape() {
        for example in schema()["examples"].as_array().unwrap() {
            let packet: SurfacePacket = serde_json::from_value(example.clone()).unwrap();
            assert!(packet.validate_shape().is_ok(), "{example}");
        }
    }

    #[test]
    fn web_requirements_in_the_schema_match_the_contract() {
        let schema = schema();
        for (surface, key) in [
            (Surface::Cli, "cli_requirements"),
            (Surface::Tui, "tui_requirements"),
            (Surface::Web, "web_requirements"),
        ] {
            let listed: BTreeSet<String> = schema["$defs"][key]["properties"]["evidence_kinds"]
                ["allOf"]
                .as_array()
                .unwrap()
                .iter()
                .map(|clause| clause["contains"]["const"].as_str().unwrap().to_string())
                .collect();
            let required: BTreeSet<String> = surface
                .required_evidence()
                .into_iter()
                .map(|kind| {
                    serde_json::to_value(kind)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string()
                })
                .collect();
            assert_eq!(listed, required, "{key}");
        }
    }

    #[test]
    fn every_surface_requires_environment_and_executable_evidence() {
        for surface in [Surface::Cli, Surface::Tui, Surface::Web] {
            let packet = SurfacePacket::output_recovery_room(surface);
            assert_eq!(packet.schema_version, CURRENT_SCHEMA_VERSION);
            assert!(packet
                .evidence_kinds
                .contains(&EvidenceKind::EnvironmentIdentity));
            assert!(packet
                .evidence_kinds
                .contains(&EvidenceKind::ExecutableCheck));
        }
    }
}
