# Design grounding

Four papers shape the contract and the judge. The code encodes their
constraints; it does not claim to reproduce their experiments.

## What the evidence says

| Paper | Finding | Consequence here |
| --- | --- | --- |
| Terminal-Bench (Merrill et al., 2026) | 89 hard terminal tasks. Each task pairs an instruction with a unique environment, a human solution, and comprehensive tests; frontier agents score below 65%. | A screen transcript is not proof. A packet must name the environment and a task-defined executable check. |
| Judging the Judges (Shi et al., 2024) | 15 judges, roughly 150k instances. Position bias is systematic, varies by judge and task, and is moderated by the quality gap. | Pairwise comparisons run in both orders. Inconsistency is reported as no winner, not as a tie. |
| Bias in the Loop (Zhao et al., 2026) | Software-engineering judges shift with presentation cues even when the artifact is unchanged; effects can change task conclusions and model rankings. | Prompt identity is evidence. Prompt edits are experimental variables, not neutral refactors. |
| TRACE (Kim et al., 2026) | Reference-free trajectory evaluation with an evidence bank; efficiency, hallucination, and adaptivity matter beyond the final answer. | Keep per-turn evidence and a bounded projection. Do not reduce evaluation to the final output. |

## What changed

- Contract v2 adds `environment_identity` and `executable_check` to every
  surface's required evidence. The v1 schema is archived at `schema/v1/`.
- Contract v3 carries the critic's running notes and caller-owned scenario
  claims, so a per-turn record (TRACE's evidence bank) travels with the
  packet instead of only the final verdict. v2 is archived at `schema/v2/`.
- `judge::pairwise` asks both orders and reports `Consistent`, `Inconsistent`,
  or `Tie`. Only `Consistent` yields a winner, and only for that pair.
- The recursive observer already rejects claims that cite turns outside the
  supplied projection. That is the evidence-bank rule at one remove.
- Observer prompts stay caller-owned, so a prompt change is visible at the call
  site instead of hidden in the crate.

## What this does not claim

- The crate is not a benchmark, a container runtime, or a semantic truth oracle.
- A consistent judge is not a correct judge. Agreement only rules out position
  bias for that pair.
- No human study was run here. The papers are the evidence; the code carries
  their constraints and the tests pin the behavior.

## References

- M. A. Merrill et al. *Terminal-Bench: Benchmarking Agents on Hard, Realistic
  Tasks in Command Line Interfaces.* arXiv:2601.11868, 2026.
- L. Shi et al. *Judging the Judges: A Systematic Study of Position Bias in
  LLM-as-a-Judge.* arXiv:2406.07791, 2024 (rev. 2025).
- Z. Zhao, A. Esmaeili, F. Fard. *Bias in the Loop: Auditing LLM-as-a-Judge for
  Software Engineering.* arXiv:2604.16790, 2026.
- W. Kim et al. *Beyond the Final Answer: Evaluating the Reasoning Trajectories
  of Tool-Augmented Agents.* arXiv:2510.02837, 2025 (rev. 2026).
