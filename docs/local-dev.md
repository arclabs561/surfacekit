# Local development workflow

The commands below are the same ones CI runs. The point is one loop: fast
feedback while editing, one full check before commit, identical proof in CI.

## Tiers

| Tier | Tools | When |
| --- | --- | --- |
| 0: always | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --locked`, `cargo doc --no-deps` | Every commit |
| 1: speed | `cargo-nextest`, `bacon` | Inner loop while editing |
| 2: supply chain | `cargo-deny`, `cargo-audit` | Before release, weekly |
| 3: mutation | `cargo-mutants` | Before a major release, not per commit |

`just check` runs tier 0, a contract-only build, and `cargo publish --dry-run`.
`just test` prefers `cargo-nextest` when installed and falls back to
`cargo test`. `just e2e` runs the opt-in live provider check and requires a
key. A library still commits `Cargo.lock` so `--locked` is meaningful in CI.

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

CI repeats tier 0 on pull requests and main. Release uses crates.io **Trusted
Publishing**: the workflow requests an OIDC token, `crates-io-auth-action`
exchanges it, and `cargo publish` runs without a long-lived secret. Configure
the publisher once at `https://crates.io/crates/<crate>/settings`.

Release also runs `cargo-semver-checks` before publishing, and uses a
`workflow_dispatch` confirmation input rather than a tag trigger so an
accidental push cannot publish. Pin third-party actions to commit SHAs, not
moving tags.

Until the publisher is configured, the release workflow fails at the auth step
with `No Trusted Publishing config found`. Publish locally with `cargo publish`
in that state. Do not add a long-lived crates.io token to the repository as a
shortcut.

`--locked` is only meaningful when `Cargo.lock` is committed. A library should
commit it; a crate that deliberately does not must drop `--locked` instead of
pretending the lock participates.

## Refinements from existing CI

The first adoption cohort showed the template must not assume one shape:

- Feature-gated crates need a matrix (`--no-default-features`, `--all-features`)
  and a separate doc-test run (`cargo test --doc`).
- Crates with a declared MSRV need a pinned-toolchain job, ideally using the
  lock so dependency drift does not masquerade as an MSRV failure.
- Binary crates may use `cargo-dist` for artifacts; libraries should not.
- Some crates publish through Trusted Publishing and semver checks already;
  the template should copy that, not reintroduce a token.
- A minimal crate may only need `check`, `test`, `clippy`, and `fmt`; adding
  doc and package stages is still cheap and catches packaging errors early.

Friction discovered in a package is recorded here and folded into the template
once two packages need the same change.

## Pilot results

Tier 0 was run on four clean packages of different shapes on 2026-10-24:

- Two single-crate libraries with no features passed every stage.
- One feature-gated library passed every stage.
- One CUDA-backed, feature-gated library passed format, test, and doc, but
  failed clippy under `--all-features`: its CUDA backend needs a toolkit the
  machine does not have.

That failure is the rule: `--all-features` is a host-dependent promise, not a
default. Use it only when every feature builds on the runner; otherwise
enumerate features and keep the all-features job on a runner that has the
required toolchain. Two packages in the survey hit this: one needs a CUDA
toolkit for its own backend, another links CUDA through a dependency's build
script.

Further refinements from the cohort:

- Skip the package stage when a crate sets `publish = false` or releases through
  `cargo-dist`; the command is a false failure there.
- Feature-gated crates keep their own matrix and MSRV jobs. A generic justfile
  must not replace a richer CI with a weaker one.
- Add a minimal-feature job (`cargo check --no-default-features`) when a crate
  gates heavy dependencies; otherwise the default build hides a broken
  contract-only path.
- A simple crate can adopt the workflow as a justfile without changing its CI;
  the two stay aligned because both run the same commands.

Adoption so far: the reference crate and two simple packages run `just check`.
Two feature-rich packages keep their existing CI, which already exceeds the
template; their patterns are the source of the refinements above.


## Adoption

Start with one reference package, then cohorts of three or four. Run the tier-0
commands on a package before adding files to it, and keep each package's own
runner (justfile, Makefile, script) rather than replacing it. Friction found in
a package becomes a change to this note, not a special case in the template.

Sources: [cargo-nextest](https://nexte.st/), [bacon](https://github.com/Canop/bacon),
[cargo-deny](https://embarkstudios.github.io/cargo-deny/), [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit),
[cargo-mutants](https://mutants.rs/).
