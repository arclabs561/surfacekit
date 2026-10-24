//! Typed, surface-neutral contracts for evaluators.
//!
//! Rust owns the meaning of these packets for CLI and TUI fixtures. The
//! serialized form is separately admitted by `eval/surface-evaluation.schema.json`
//! so browser evaluators can exchange the same evidence without importing Rust.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const OUTPUT_RECOVERY_ROOM: &str = "output-recovery-room";

/// Current packet revision. v2 adds environment identity and executable
/// checks, because a rendered screen alone does not prove a terminal task.
pub const CURRENT_SCHEMA_VERSION: u8 = 2;

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
    /// Populated by the observer. A declaration-only packet may leave this
    /// empty; [`SurfacePacket::validate_complete`] then fails.
    pub evidence: Vec<Evidence>,
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
            evidence: Vec::new(),
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
            evidence: Vec::new(),
        }
    }

    /// Version, observation authority, required evidence, and at least one
    /// claim. Scenario-specific rules belong to [`SurfacePacket::validate`].
    pub fn validate_shape(&self) -> Result<(), &'static str> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err("unknown surface packet schema version");
        }
        if self.observation != self.surface.observation() {
            return Err("surface has the wrong observation authority");
        }
        if !self
            .evidence_kinds
            .is_superset(&self.surface.required_evidence())
        {
            return Err("surface packet omits required evidence");
        }
        if self.claims.is_empty() {
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
