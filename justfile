set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
    @just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

# nextest is faster and gives per-test isolation; fall back to cargo test.
test:
    @if command -v cargo-nextest >/dev/null; then cargo nextest run --locked; else cargo test --locked; fi

# Contract-only build, no PTY or judge dependencies.
minimal:
    cargo check --no-default-features
    cargo test --no-default-features --locked

# Opt-in live provider check; requires OPENROUTER_API_KEY.
e2e:
    cargo test --test live_pairwise -- --ignored --nocapture --test-threads=1

doc:
    cargo doc --no-deps --locked

deny:
    cargo deny check

audit:
    cargo audit

semver:
    cargo semver-checks check-release

package:
    cargo publish --dry-run --allow-dirty --locked

check: fmt-check lint test minimal doc package
