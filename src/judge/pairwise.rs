//! Order-swapped pairwise judging.
//!
//! Position bias in LLM judges is systematic, not random: the same pair can
//! receive different verdicts when the order changes, and the effect varies by
//! judge and task (Shi et al., 2024). Code judges are additionally sensitive to
//! presentation cues even when the artifact is unchanged (Zhao et al., 2026).
//!
//! A single comparison is therefore not a verdict. This module asks the same
//! question in both orders and reports whether the answers agree. Callers must
//! treat [`Agreement::Inconsistent`] as no winner, not as a tie.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    First,
    Second,
    Tie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub side: Side,
    pub rationale: String,
}

impl Verdict {
    #[must_use]
    pub fn new(side: Side, rationale: impl Into<String>) -> Self {
        Self {
            side,
            rationale: rationale.into(),
        }
    }
}

/// A caller-owned judge. Implementations own transport, prompts, and parsing.
pub trait PairwiseJudge {
    fn compare(&self, first: &str, second: &str) -> Verdict;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    /// Both orders picked the same original candidate.
    Consistent,
    /// The verdict followed the position, so it carries no preference.
    Inconsistent,
    /// Both orders declined to choose.
    Tie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterbalanced {
    /// Verdict for `(left, right)`.
    pub forward: Verdict,
    /// Verdict for `(right, left)`, mapped back to the original labels.
    pub reverse: Verdict,
    pub agreement: Agreement,
}

impl Counterbalanced {
    /// A winner is only reported when both orders agree.
    #[must_use]
    pub fn winner(&self) -> Option<Side> {
        match self.agreement {
            Agreement::Consistent => Some(self.forward.side),
            Agreement::Inconsistent | Agreement::Tie => None,
        }
    }
}

/// Ask the same question in both orders.
#[must_use]
pub fn counterbalance(judge: &dyn PairwiseJudge, left: &str, right: &str) -> Counterbalanced {
    let forward = judge.compare(left, right);
    let raw_reverse = judge.compare(right, left);
    let reverse = Verdict {
        side: flip(raw_reverse.side),
        rationale: raw_reverse.rationale,
    };
    let agreement = match (forward.side, reverse.side) {
        (Side::Tie, Side::Tie) => Agreement::Tie,
        (a, b) if a == b => Agreement::Consistent,
        _ => Agreement::Inconsistent,
    };
    Counterbalanced {
        forward,
        reverse,
        agreement,
    }
}

const fn flip(side: Side) -> Side {
    match side {
        Side::First => Side::Second,
        Side::Second => Side::First,
        Side::Tie => Side::Tie,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysFirst;

    impl PairwiseJudge for AlwaysFirst {
        fn compare(&self, _first: &str, _second: &str) -> Verdict {
            Verdict::new(Side::First, "first looks better")
        }
    }

    struct PrefersMarker(&'static str);

    impl PairwiseJudge for PrefersMarker {
        fn compare(&self, first: &str, second: &str) -> Verdict {
            if first.contains(self.0) {
                Verdict::new(Side::First, "marker present")
            } else if second.contains(self.0) {
                Verdict::new(Side::Second, "marker present")
            } else {
                Verdict::new(Side::Tie, "no marker")
            }
        }
    }

    struct AlwaysTie;

    impl PairwiseJudge for AlwaysTie {
        fn compare(&self, _first: &str, _second: &str) -> Verdict {
            Verdict::new(Side::Tie, "indistinguishable")
        }
    }

    #[test]
    fn a_position_biased_judge_reports_no_winner() {
        let outcome = counterbalance(&AlwaysFirst, "left", "right");
        assert_eq!(outcome.agreement, Agreement::Inconsistent);
        assert_eq!(outcome.winner(), None);
    }

    #[test]
    fn a_content_judge_agrees_across_orders() {
        let judge = PrefersMarker("GOOD");
        let outcome = counterbalance(&judge, "GOOD candidate", "BAD candidate");
        assert_eq!(outcome.agreement, Agreement::Consistent);
        assert_eq!(outcome.winner(), Some(Side::First));

        let outcome = counterbalance(&judge, "BAD candidate", "GOOD candidate");
        assert_eq!(outcome.winner(), Some(Side::Second));
    }

    #[test]
    fn two_ties_are_a_tie_not_a_winner() {
        let outcome = counterbalance(&AlwaysTie, "a", "b");
        assert_eq!(outcome.agreement, Agreement::Tie);
        assert_eq!(outcome.winner(), None);
    }
}
