# surfacekit

Surface-neutral observation and evidence packets for agentic testing.

The code is extracted from the terminal harness in [piku](https://github.com/arclabs561/piku) (MIT). The goal is one contract and one observer loop shared by terminal, subprocess, and browser evaluators, so evidence can move between a product, a judge, a notebook, and a comparator without each project re-implementing PTY parsing.

## Install

Default build (contract, PTY observation, scenarios, judge):

```toml
surfacekit = "0.7"
```

Contract only, without PTY, VT100, or judge dependencies:

```toml
surfacekit = { version = "0.7", default-features = false }
```

## Layers

| Module | Feature | Role |
| --- | --- | --- |
| `contract` | always | `Surface`, `Observation`, `EvidenceKind`, `Claim`, `Evidence`, `SurfacePacket`, JSON schema. |
| `notes` | always | `Notebook`, `Note`, `NoteDraft`, `NoteKind`: a critic's dated, append-only running notes, carried in the packet. |
| `term` | `term` | Real PTY handles, `SessionIo`, capability profiles, emulator seam, VT100 observers, workspace diff, recording. |
| `scenario` | `scenario` | Actors, action resolutions, encode, timeline, replay, regression cases, trials, evidence records, bounded live critique, stateless (`Critic`) or with a running notebook (`NotingCritic`). |
| `judge` | `judge` | Deterministic checks, a provider-neutral judge seam, and order-swapped pairwise judging. |

`scenario` requires `term`; `judge` requires `scenario`. The modules are not
separate packages because a published crate cannot depend on unpublished path
crates.

## Status

`0.1.0` through `0.7.0` are published. Run local checks with `cargo test`.
Two live OpenRouter tests are opt-in. CI runs format, clippy, `cargo test`, a
contract-only build, docs, and a publish dry-run.

## Boundary

This crate records observations and checks claims. It does not own product TUIs, provider
transports, durable notebooks, comparison math, or coordination. Those stay
with the projects that already implement them.

## Known couplings

- The named scenario preset is `output-recovery-room`; `SurfacePacket::new`
  builds other scenarios but does not validate scenario-specific claim rules.
- Judge response keys default to `judge_observations` / `subject_observations`;
  `observe_with_keys` overrides them, and the prompt must match.
- The terminal profile is declared, not probed; a mismatch is evidence, not a
  type error.
- `eval/` keeps the historical schemas used by tests; `schema/` is the
  published copy (v3), with v1 and v2 archived under `schema/v1/` and
  `schema/v2/`.

## Development

`just check` runs format, clippy, nextest (or `cargo test`), a contract-only
build, docs, and a publish dry-run. `just e2e` runs the live OpenRouter pair.
CI runs the same stages; see `docs/local-dev.md`. The design constraints from
the literature are in `docs/grounding.md`.

## License

MIT. Derived from piku, copyright (c) 2026 Henry Wallace. See `LICENSE`.
