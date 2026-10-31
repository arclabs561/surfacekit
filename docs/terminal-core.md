# Design: terminal core abstraction

Status: draft. Scope: `surfacekit-contract` + `surfacekit-term` + `surfacekit-scenario`.

## Problem

The crate can observe one emulator (`vt100`) through one transport
(`portable-pty`). It cannot say what a terminal *supports*, cannot separate a
typed character from a paste, and cannot record an action at more than one
resolution. Consumers therefore duplicate the loop or silently assume `vt100`
behavior.

## Context

Consumers and what they actually need:

| Consumer | Needs |
| --- | --- |
| piku TUI | resize, alternate screen, truecolor, bracketed paste, kitty keyboard, OSC 133/7, scrollback |
| piku web (xterm.js) | truecolor, mouse, focus events, browser events, screenshots |
| agentic tests | deterministic grid, key events, resize, stable scrollback |
| real terminals (Ghostty and peers) | the same byte protocol and capability profile; they are frontends, not backends |

## Prior art

| Project | Boundary | Lesson |
| --- | --- | --- |
| alacritty/vte | pure parser, `Perform` sink, zero state | split parse from state; the sink owns policy |
| wezterm `termwiz` + `term` | escape parser → terminal state → GUI | key encoding is profile-parameterized; do not embed the GUI |
| xterm.js | parser + buffer + renderer, headless entry point | a headless core is viable; markers are buffer events |
| asciinema v3 | capture format + player | `o`/`i`/`r`/`m`/`x` codes, monotonic time, markers as events |
| pexpect | pty + expect matcher | byte-level only; planned actions must be re-derived |
| Playwright trace | versioned trace + viewer | version the format; replay is a first-class artifact |
| harbor / Terminal-Bench | tmux transport + evidence | keys vs paste split; byte capture is not observation |
| ghostty | protocol/encoding only | profile-driven key encoding without embedding the GUI |

## Options considered

1. **Extend `Observation` with more variants.** Rejected: it mixes "who
   captures evidence" with "what the renderer supports".
2. **One backend per frontend (Ghostty, xterm.js, vt100).** Rejected: the
   frontends do not implement a Rust observation loop, and embedding a GUI
   inherits its grid and protocol version.
3. **Separate axes plus declared profiles.** Chosen.

## Chosen approach

Four independent concepts:

- `TerminalProfile`: declared capabilities: dimensions, color depth,
  alternate screen, bracketed paste, mouse, SGR mouse, focus events,
  hyperlinks, kitty keyboard, OSC 133/7, scrollback depth. Presets `vt100`,
  `xterm`, `ghostty`.
- `TerminalEmulator`: `feed`, `resize`, `screen_text`, `profile`.
  `Vt100Emulator` is the first implementation; `TerminalObserver::resize` now
  uses `Screen::set_size`.
- `ActionPlan` / `Step`: the resolution ladder: `Plan`, `Command`, `Line`,
  `Text`, `Paste`, `KeyChord`, `PhysicalKey`, `Wait`, `Raw`. `encode` maps a
  plan to bytes using the profile.
- `Timeline`: monotonic events: `output`, `action`, `key`, `resize`,
  `marker`, `check`, `critique`. `Replay` re-feeds actions into a fresh
  emulator and reports divergence. `Case` is the regression quadruple
  `init / act / check / teardown`, with `live_dependencies` named explicitly.

## Corrections from replay and optimization research

- **Append order orders events; timestamps only pace them.** Replay iterates
  the recorded sequence and never sorts by `at_ms`.
- **Record inputs, re-derive outputs.** Replay feeds actions into a fresh
  emulator and compares state; recorded output is evidence, not replay input.
- **Comparison mode is explicit.** `MatchMode::{Skip, Contains, Exact}` makes
  the replay contract visible.
- **Promotion needs an oracle, provenance, and a minimized slice.** `Case`
  carries `CheckSpec`, `Provenance`, `Reward`, and named `live_dependencies`.
- **Trials are first-class.** `TrialSet` reports pass rate, unbiased pass@k,
  and mean cost; optimizers consume trials, not a single run.

## Corrections from primary specs

- **Typed text is not paste.** `Text` never gets bracketed markers; only
  `Paste` does, and only when the profile supports it. Payload controls are
  stripped by the caller.
- **Kitty `CSI u` is opt-in per flag.** Ctrl+C becomes `CSI 99;5u` once the
  flag is active and no longer raises SIGINT; Enter/Tab/Backspace stay legacy.
  Our encoder applies `CSI u` only to character chords on kitty profiles.
- **SGR mouse (1006) is an encoding, not an enabler.** Reporting still needs
  1000/1002/1003, so the profile carries `mouse` and `sgr_mouse` separately.
- **Alternate screen scrollback is not normative.** 1049 is the full-screen
  choice; discarded contents must not be restored.
- **Resize carries no size.** SIGWINCH only signals; the reader must re-query
  `TIOCGWINSZ`. Reflow is unspecified and is not a capability bit.
- **DECSTBM homes the cursor.** Emit it, then an explicit CUP; never rely on
  the prior cursor.
- **Unicode width has no query sequence.** Width is per-cell and varies for
  emoji ZWJ sequences; do not assume one width function.

## Non-goals

- No GUI terminal embedding (Ghostty, libghostty, xterm.js runtime).
- No renderer, font stack, or screenshot pipeline.
- No live-site or live-model replay claim; those are `live_dependencies`.
- No capability probing in this slice; profiles are declarations.
- No second emulator implementation until a consumer needs one.

## Tradeoffs

- `screen_text()` is lossy; styled cells and cursor state stay on the concrete
  observer.
- Declared profiles can be wrong; the type makes the claim explicit, not true.
- `PtyHandle` stays concrete until a second transport is needed.
- Recording at keystroke resolution costs volume; replay can use coarser
  actions, but the fine record is what makes a failure reproducible.

## Implementation plan

1. `TerminalProfile`, `ColorDepth`, presets (reversible).
2. `TerminalEmulator`, `Vt100Emulator`, observer resize (reversible).
3. `ActionPlan`, `Step`, `encode`, typed `Timeline` (reversible).
4. `Replay` and `Case` (reversible until a corpus format is published).
5. Embed the profile in `SurfacePacket` as schema v3 once a packet consumer
   needs it (one-way for the JSON contract).

## Decision gates

- A second emulator consumer appears: re-open `screen_text` before it hardens.
- A consumer needs cursor/style through the trait: replace it with a typed
  snapshot.
- A declared profile disagrees with observed behavior: add probing before
  trusting profiles in evidence.
- Replay divergence is dominated by timing rather than state: add a timing
  model instead of tightening assertions.

## Open questions

- Is `scrollback` a depth, a retention policy, or an evidence bound?
- Should input encoding live in `ActionPlan` or behind a separate encoder
  trait?
- Does the web surface need its own profile, or can xterm.js share `xterm`?
