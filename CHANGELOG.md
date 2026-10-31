# Changelog

## Unreleased

### Changed (breaking)

- `SurfacePacket` gains `recording: Option<RecordingMetadata>`, typed to the
  schema's `recording_metadata` (with `RecordingStatus`, `Inspectability`,
  `SourceArtifact` and `Retention`). Deserializing a packet previously
  dropped its `recording`, so a re-serialized packet lost it.
  `validate_shape` requires a non-empty `reason` when the recording is
  incomplete or uninspectable, at least one source artifact, and non-empty
  artifact references, as the schema does.
- `SurfacePacket` is `#[non_exhaustive]`: build it with `SurfacePacket::new`
  or `SurfacePacket::output_recovery_room` and set fields afterwards, so
  later schema fields are not breaking.

### Fixed

- `TrialSet::pass_at_k` no longer overflows `u128` binomial coefficients (a
  panic in debug builds, a wrapped wrong value in release) at sizes such as
  n = 200, k = 100, the setting Chen et al. (2021) report. It uses their
  product form `1 - prod_{i=n-c+1}^{n} (1 - k/i)`.

## 0.7.1 - 2026-10-27

- `CommandPty::signal` sends a signal, given as a raw number such as
  `libc::SIGUSR1`, to the child, so a consumer can test its program's own
  signal handling (a dev-reload or terminal-restore handler) without
  reaching past `CommandPty` to rexpect. Taking a number keeps nix's version
  out of this crate's API.

## 0.7.0 - 2026-10-27

- Breaking: packet schema v3. `SurfacePacket` gains `notes` (a critic's
  running notes) and `scenario_claims` (`scope:claim` strings owned by a
  caller-defined scenario); `LoopReport` gains `notes`. Code that builds
  either struct with a literal must add the fields. `validate_shape` accepts
  v2 and v3 (`SUPPORTED_SCHEMA_VERSIONS`); a v2 packet may not carry the new
  fields. The v2 schema is archived at `schema/v2/`.
- The v3 schema admits any `scenario_id`; the seven output-recovery claims
  are required only when the id is `output-recovery-room`, which matches what
  `validate_shape` already did. It also requires `environment_identity` and
  `executable_check` on every surface, as `required_evidence` did since v2
  while the v2 schema did not. A packet may omit `evidence`, as the schema
  always allowed; it deserializes as empty.
- `notes::Notebook`: dated, numbered, append-only notes (`observation`,
  `expectation_gap`, `friction`, `delight`, `question`). A revision is a new
  note that names the one it supersedes; `current()` hides superseded notes,
  `render(limit)` gives bounded prompt text with ids.
- `NotingCritic` and `run_actor_with_notebook`: after each action the critic
  reads the screen, the action, and its current notes, and appends drafts.
  Each note records the step and the measured latency of the call that wrote
  it. `VerdictNotes` runs a stateless `Critic` in the same loop;
  `run_actor_with_critic` is unchanged.

- `CommandPty` teardown is bounded: dropping it sends SIGTERM, then SIGKILL
  after 1s, and keeps reading the master while it waits (5s cap). Before, it
  fell through to rexpect's drop, which loops until the child is gone, and a
  child with a full terminal output buffer could not finish exiting while
  nothing read the master, so one failed assertion could hang a test suite.
- `CommandPty::wait_until` drains output into a `ScreenObserver` until a
  screen predicate holds or a timeout passes.
- `CommandPty::exit_code` (non-blocking, remembered once observed),
  `wait_for_exit` (bounded, reads output while waiting), and `master` (a
  handle for terminal ioctls).

## 0.6.0 - 2026-10-24

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

## 0.5.1 - 2026-10-24

- `Case::ready_when` records the stable screen marker that means a session is
  ready. `replay_case` waits for it before sending the recorded actions; without
  it, replay against a fresh TUI sends input before the prompt exists.


## 0.5.0 - 2026-10-24

- `Actor`, `ScriptedActor`, and `ExpectActor`, plus `run_actor` with drain,
  pacing, timeout, and exit reporting.
- Generic `CommandPty` and `SessionIo`, separate from the piku-specific
  recorder; nonblocking reads and bounded write retries.
- `Case::from_timeline` and `replay_case` re-execute a promoted case against a
  fresh session and report missing checks as divergence.
- `Event::Exit`, `LoopReport.error`, and replay `error` reporting.
- End-to-end test: real PTY fixture, actor, timeline, promoted case, replay,
  and a mutated check that diverges.


## 0.4.0 - 2026-10-24

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
