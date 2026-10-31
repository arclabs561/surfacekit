//! Repeated trials over a case.
//!
//! A single run is not a measurement. Trials carry seed and temperature so a
//! pass rate, pass@k, cost, and variance can be reported honestly.

use crate::scenario::case::Reward;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trial {
    pub case_id: String,
    pub seed: u64,
    pub temperature: Option<f64>,
    pub reward: Reward,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrialSet {
    pub trials: Vec<Trial>,
}

impl TrialSet {
    #[must_use]
    pub fn new(trials: Vec<Trial>) -> Self {
        Self { trials }
    }

    #[must_use]
    pub fn pass_rate(&self) -> Option<f64> {
        if self.trials.is_empty() {
            return None;
        }
        let passed = self
            .trials
            .iter()
            .filter(|trial| trial.reward.passed)
            .count();
        Some(passed as f64 / self.trials.len() as f64)
    }

    /// Unbiased pass@k over the recorded trials: `1 - C(n-c,k)/C(n,k)`.
    #[must_use]
    pub fn pass_at_k(&self, k: usize) -> Option<f64> {
        let n = self.trials.len();
        if k == 0 || n < k {
            return None;
        }
        let c = self
            .trials
            .iter()
            .filter(|trial| trial.reward.passed)
            .count();
        if n - c < k {
            return Some(1.0);
        }
        // Product form from Chen et al. (2021), "Evaluating Large Language
        // Models Trained on Code", Figure 3: C(n-c,k)/C(n,k) equals
        // prod_{i=n-c+1}^{n} (1 - k/i), with no binomial coefficients to
        // overflow at the paper's n = 200.
        let fail_all: f64 = (n - c + 1..=n).map(|i| 1.0 - k as f64 / i as f64).product();
        Some(1.0 - fail_all)
    }

    #[must_use]
    pub fn mean_cost(&self) -> Option<f64> {
        let costs = self
            .trials
            .iter()
            .filter_map(|trial| trial.reward.cost_usd)
            .collect::<Vec<_>>();
        if costs.is_empty() {
            None
        } else {
            Some(costs.iter().sum::<f64>() / costs.len() as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(passed: bool, cost: f64) -> Trial {
        Trial {
            case_id: "case".into(),
            seed: 1,
            temperature: Some(0.0),
            reward: Reward {
                passed,
                cost_usd: Some(cost),
                ..Reward::default()
            },
        }
    }

    #[test]
    fn pass_rate_and_cost_are_reported() {
        let set = TrialSet::new(vec![trial(true, 0.1), trial(false, 0.3)]);
        assert_eq!(set.pass_rate(), Some(0.5));
        assert_eq!(set.mean_cost(), Some(0.2));
    }

    #[test]
    fn pass_at_k_uses_the_unbiased_estimator() {
        let all_pass = TrialSet::new(vec![trial(true, 0.0), trial(true, 0.0)]);
        assert_eq!(all_pass.pass_at_k(1), Some(1.0));

        let none_pass = TrialSet::new(vec![trial(false, 0.0), trial(false, 0.0)]);
        assert_eq!(none_pass.pass_at_k(1), Some(0.0));

        let half = TrialSet::new(vec![trial(true, 0.0), trial(false, 0.0)]);
        assert_eq!(half.pass_at_k(1), Some(0.5));
        assert_eq!(half.pass_at_k(2), Some(1.0));
        assert_eq!(half.pass_at_k(3), None);
    }

    #[test]
    fn pass_at_k_handles_the_papers_n200_setting() {
        // Chen et al. (2021) estimate pass@k from n = 200 samples. C(200, 100)
        // is about 9e58, beyond u128. Expected values are the exact rationals
        // 1 - C(197, 100) / C(200, 100) and 1 - C(190, 10) / C(200, 10).
        let set = |c: usize| TrialSet::new((0..200).map(|i| trial(i < c, 0.0)).collect());
        let at_100 = set(3).pass_at_k(100).unwrap();
        assert!((at_100 - 0.876_884_422_110_552_7).abs() < 1e-12, "{at_100}");
        let at_10 = set(10).pass_at_k(10).unwrap();
        assert!((at_10 - 0.408_547_866_081_417_13).abs() < 1e-12, "{at_10}");
    }
}
