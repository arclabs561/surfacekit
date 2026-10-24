# Local development workflow

The commands below are the same ones CI runs. The point is one loop: fast
feedback while editing, one full check before commit, identical proof in CI.

## Tiers

| Tier | Tools | When |
| --- | --- | --- |
| 0 — always | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --locked`, `cargo doc --no-deps` | Every commit |
| 1 — speed | `cargo-nextest`, `bacon` | Inner loop while editing |
| 2 — supply chain | `cargo-deny`, `cargo-audit` | Before release, weekly |
| 3 — mutation | `cargo-mutants` | Before a major release, not per commit |

`just check` runs tier 0 and `cargo publish --dry-run`. `just test` prefers
`cargo-nextest` when installed and falls back to `cargo test`. A library still
commits `Cargo.lock` so `--locked` is meaningful in CI.

## Inner loop

```sh
cargo nextest run          # or: cargo test
bacon test                 # watch mode; missing on some machines
cargo check --all-targets  # fastest type loop
```

`cargo check` answers "does it compile", `clippy` answers "is it idiomatic",
`nextest` answers "does it behave". Keep them separate; do not hide one inside
another.

## Before commit

```sh
just check
```

If a package cannot run one stage, record why in its own docs instead of
deleting the stage. Green output is not proof: read warnings, skips, and
`ignored` counts.

## CI and release

CI repeats tier 0 on pull requests and main. Release runs on a `v*` tag, checks
that the tag matches `Cargo.toml`, and publishes with `CARGO_REGISTRY_TOKEN`.
Never publish from a dirty tree; `cargo publish` reads committed state only if
you pass `--allow-dirty` deliberately.

## Adoption

Start with one reference package, then cohorts of three or four. Run the tier-0
commands on a package before adding files to it, and keep each package's own
runner (justfile, Makefile, script) rather than replacing it. Friction found in
a package becomes a change to this note, not a special case in the template.

Sources: [cargo-nextest](https://nexte.st/), [bacon](https://github.com/Canop/bacon),
[cargo-deny](https://embarkstudios.github.io/cargo-deny/), [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit),
[cargo-mutants](https://mutants.rs/).
