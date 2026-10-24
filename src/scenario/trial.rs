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
        Some(1.0 - combination(n - c, k) as f64 / combination(n, k) as f64)
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

fn combination(n: usize, k: usize) -> u128 {
    if k > n {
        return 0;
    }
    (0..k).fold(1u128, |acc, i| acc * (n - i) as u128 / (i + 1) as u128)
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
}
