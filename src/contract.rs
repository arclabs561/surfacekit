//! Typed, surface-neutral contracts for evaluators.
//!
//! Rust owns the meaning of these packets for CLI and TUI fixtures. The
//! serialized form is separately admitted by `eval/surface-evaluation.schema.json`
//! so browser evaluators can exchange the same evidence without importing Rust.

use serde::Serialize;
use std::collections::BTreeSet;

pub const OUTPUT_RECOVERY_ROOM: &str = "output-recovery-room";

/// Current packet revision. v2 adds environment identity and executable
/// checks, because a rendered screen alone does not prove a terminal task.
pub const CURRENT_SCHEMA_VERSION: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Cli,
    Tui,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Observation {
    BoundedSubprocess,
    ManagedPty,
    BoundedPlaywright,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SurfacePacket {
    pub schema_version: u8,
    pub scenario_id: &'static str,
    pub surface: Surface,
    pub observation: Observation,
    pub evidence_kinds: BTreeSet<EvidenceKind>,
    pub claims: BTreeSet<Claim>,
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
            scenario_id: OUTPUT_RECOVERY_ROOM,
            surface,
            observation: surface.observation(),
            evidence_kinds: surface.required_evidence(),
            claims: REQUIRED_CLAIMS.iter().copied().collect(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err("unknown surface packet schema version");
        }
        if self.scenario_id != OUTPUT_RECOVERY_ROOM {
            return Err("unknown shared evaluation scenario");
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
        if !self
            .claims
            .is_superset(&REQUIRED_CLAIMS.iter().copied().collect())
        {
            return Err("surface packet omits a required scenario claim");
        }
        Ok(())
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
