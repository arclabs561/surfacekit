# surfacekit

Surface-neutral observation and evidence packets for agentic testing.

The code is extracted from the terminal harness in [piku](https://github.com/arclabs561/piku) (MIT). The goal is one contract and one observer loop shared by terminal, subprocess, and browser evaluators, so evidence can move between a product, a judge, a notebook, and a comparator without each project re-implementing PTY parsing.

## Layers

One published crate, four internal modules:

| Module | Role |
| --- | --- |
| `contract` | `Surface`, `Observation`, `EvidenceKind`, `Claim`, `SurfacePacket`, and the JSON schema. Serde only, no I/O. |
| `term` | Real PTY handle, VT100 screen observers, workspace diff, asciicast/JSONL recording. |
| `scenario` | Clause-bound scenarios, terminal decisions, append-only evidence records. |
| `judge` | Deterministic checks and a provider-neutral judge seam. Prompts are caller data. |

The modules are not separate packages because a published crate cannot depend on unpublished path crates.

## Status

Local scaffold. Not published. `cargo test` runs 30 tests: 2 contract, 4 term, 20 scenario, 4 judge. `cargo publish --dry-run` packages 33 files and verifies the build.

## Boundary

This crate observes and proves. It does not own product TUIs, provider transports, durable notebooks, comparison math, or coordination. Those stay with the projects that already implement them.

## Development

`just check` runs format, clippy, nextest (or `cargo test`), docs, and a publish dry-run. CI runs the same stages; see `docs/local-dev.md`. The design constraints from the literature are in `docs/grounding.md`.

## Known couplings

- Ledger fields retain `piku_*` names. Renaming them is a ledger schema revision, not a cosmetic edit.
- The judge parses `piku_observations` from the model response. The prompt is caller-supplied; the response keys are not yet configurable.
- `eval/` holds the original schemas and fixtures used by tests. `schema/` is the publishable copy.

## License

MIT. Derived from piku, copyright (c) 2026 Henry Wallace. See `LICENSE`.
