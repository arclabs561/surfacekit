# Changelog

## 0.3.0 - unreleased

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
