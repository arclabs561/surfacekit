# Design: agent-driven terminal evaluation loop

Status: draft. Actor loop and session replay implemented; live critique and optimizer open.

## Purpose

Agents drive real terminal tools, critique what they observe, annotate failures
into regression cases, and feed optimization loops. The unit is a session
timeline, not a final answer. A run becomes reusable evidence only when it can
be replayed, labeled, and compared.

## Loop

`actor → session → timeline → critique → annotation → case → trials → optimizer → actor`

Each stage has a type, so a consumer can enter at any point:

| Stage | Type | State |
| --- | --- | --- |
| Actor | `Actor`, `ScriptedActor`, `ExpectActor` | implemented |
| Session | `TerminalProfile`, `Vt100Emulator`, `CommandPty`, `SessionIo` | implemented |
| Action | `ActionPlan`, `Step`, `Resolution`, `encode` | implemented |
| Timeline | `Timeline`, `Event` | implemented |
| Critique | `Event::Check`, `Event::Critique`, `VerificationProvenance` | primitives only |
| Annotation | `Event::Marker`, `CheckSpec`, `Case` | implemented |
| Replay | `replay`, `replay_case`, `MatchMode`, `Divergence` | implemented |
| Trials | `Trial`, `TrialSet`, `pass_at_k` | implemented |
| Optimization | `Reward` consumed by an external optimizer | open here |

## Gate evidence

The loop is proven on piku's real TUI: `ExpectActor` waits for the prompt,
sends a message, and waits for the scripted response. The timeline is promoted
to a `Case` with a `ready_when` marker; replaying it against a fresh piku process
passes, and a mutated check produces a divergence. A shell fixture covers the
same loop without a product dependency. The isolated piku PTY suite passes
25/25 with this test included.

## What the research changed

- **Append order orders events; `at_ms` only paces them.** The timeline is an
  append-only sequence, following asciinema and rr. Replay iterates the
  sequence, never sorts by timestamp.
- **Record inputs, re-derive outputs.** A replay feeds actions into a fresh
  emulator and compares screen state; recorded output is evidence, not input.
- **Match modes are explicit.** `Skip`, `Contains`, and `Exact` make the
  comparison contract visible instead of hiding it in an assertion helper.
- **Promotion needs an oracle, provenance, and a minimized slice.** `Case`
  carries `CheckSpec`, `Provenance`, `Reward`, and named `live_dependencies`.
  A run without an expectation is not a regression case.
- **Trials are first-class.** A single run is not a measurement. `TrialSet`
  reports pass rate, unbiased pass@k, and mean cost; optimizers consume trials,
  never a promoted case's single run.
- **Verification provenance is typed.** `Program`, `Judge`, and `Human` are
  distinct; a judge verdict cannot masquerade as an executable check.

## Ownership

| Concern | Owner |
| --- | --- |
| Actor, session, action, timeline, replay, case | `surfacekit` |
| Comparison and effect sizes | `ablate` |
| Orchestration, budgets, fan-out | `mitosis` |
| Durable corpus and notebook | `agentbook` |
| Visual judge transport | `ai-visual-test` |
| Tools under test | piku, wish, other TUIs |

## Non-goals

- No claim that agent users are equivalent to human users.
- No benchmark and no leaderboard.
- No optimizer; the crate emits trials and rewards, it does not search.
- No durable dataset store; annotations are portable, storage is external.
- No GUI embedding (Ghostty, xterm.js runtime).

## Decision gates

- An actor cannot produce a reproducible timeline on one piku scenario: fix
  the actor/timing model before adding more consumers.
- A failed run cannot be promoted to a replaying `Case` without manual
  rewriting: the case format is wrong.
- Trial variance swamps the effect being optimized: report it and stop, rather
  than tuning the judge.
- A second consumer needs a different event shape: version the timeline rather
  than adding optional fields silently.

## Open questions

- Who owns live critique latency budgets: the actor loop or the judge seam?
- Should annotations be immutable events with supersession, or mutable labels?
- Where do model/prompt hashes come from when the harness does not control the
  provider?
