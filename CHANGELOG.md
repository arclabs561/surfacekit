# Changelog

## 0.6.0 - unreleased

- `Critic` trait and `CritiqueContext`: a live critic reads the same screen the
  actor acted on and returns an optional verdict.
- `CritiqueBudget` and `run_actor_with_critic`: the actor loop owns the latency
  budget. It measures each critic call and records an over-budget critique
  rather than dropping it. It cannot preempt a synchronous critic; preemption
  is a separate gate.
- `Event::Critique` now carries optional `latency_ms` and `within_budget`.
  Records captured before 0.6.0 deserialize with both absent, which means "not
  measured", never "zero".
- Open question resolved in the design record: the actor loop owns the live
  critique latency budget, not the judge seam.

## 0.5.1 - unreleased

- `Case::ready_when` records the stable screen marker that means a session is
  ready. `replay_case` waits for it before sending the recorded actions; without
  it, replay against a fresh TUI sends input before the prompt exists.


## 0.5.0 - unreleased

- `Actor`, `ScriptedActor`, and `ExpectActor`, plus `run_actor` with drain,
  pacing, timeout, and exit reporting.
- Generic `CommandPty` and `SessionIo`, separate from the piku-specific
  recorder; nonblocking reads and bounded write retries.
- `Case::from_timeline` and `replay_case` re-execute a promoted case against a
  fresh session and report missing checks as divergence.
- `Event::Exit`, `LoopReport.error`, and replay `error` reporting.
- End-to-end test: real PTY fixture, actor, timeline, promoted case, replay,
  and a mutated check that diverges.


## 0.4.0 - unreleased

- `TerminalProfile` capability presets (`vt100`, `xterm`, `ghostty`) and the
  `TerminalEmulator` seam with a `Vt100Emulator` implementation.
- Action resolution ladder: `Plan`, `Command`, `Line`, `Text`, `Paste`,
  `KeyChord`, `PhysicalKey`, `Wait`, `Raw`, plus profile-aware `encode`.
- Typed `Timeline` events with monotonic append order; replay with explicit
  `MatchMode::{Skip, Contains, Exact}` and divergence reporting.
- Regression `Case` quadruple (`init`/`act`/`check`/`teardown`) with
  `Provenance`, `Reward`, and named live dependencies.
- `Trial`/`TrialSet` with pass rate, unbiased pass@k, and mean cost.
- Design records for the terminal core and the agent-driven evaluation loop.


## 0.3.0 - 2026-10-24

- `SurfacePacket::new` builds packets for scenarios other than the preset;
  `validate_shape` checks version, observation authority, required evidence,
  and a non-empty claim set. `validate` remains the preset-specific check.
- `observe_with_keys` / `ObserverKeys` let a caller name the judge and subject
  response fields without forking the observer. Defaults are unchanged.
- `validate_complete` now validates shape plus payload coverage, so it applies
  to generic packets as well as the preset.

## 0.2.0 - 2026-10-24

- Contract v2 adds `environment_identity` and `executable_check` to required
  evidence for every surface.
- `Evidence { kind, payload }` carries observed artifacts;
  `SurfacePacket::validate_complete` requires every declared kind to have a
  non-null payload and rejects undeclared kinds.
- Feature flags: `contract` (default always), `term`, `scenario`, `judge`.
  `default-features = false` builds the contract without PTY dependencies.
- Renamed product-specific identifiers to `subject_*`; no `piku` identifiers
  remain in `src/`.
- Added the counterbalanced pairwise judge and a live OpenRouter e2e pinned to
  `deepseek/deepseek-v4.1-flash-1017`.
- Added a schema-drift test that checks every `EvidenceKind` and the schema
  version against `schema/surface-evaluation.schema.json`.

## 0.1.0 - 2026-10-24

- First extraction: contract, PTY/VT100 observation, scenarios, deterministic
  checks, and the recursive observer.
