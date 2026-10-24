//! Live OpenRouter check for the counterbalanced pairwise judge.
//!
//! Opt-in: `cargo test --test live_pairwise -- --ignored --nocapture`.
//! Requires `OPENROUTER_API_KEY`. The model is pinned so runs are comparable.

use serde_json::{json, Value};
use surfacekit::judge::{counterbalance, Agreement, PairwiseJudge, Side, Verdict};

const MODEL: &str = "deepseek/deepseek-v4.1-flash-1017";
const SYSTEM: &str = "You compare two short answers to the same task. Reply with exactly one token: A if the first answer is better, B if the second answer is better, or TIE. No other text.";

struct OpenRouter {
    key: String,
    client: reqwest::blocking::Client,
}

impl OpenRouter {
    fn new() -> Self {
        let key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set for the live judge test");
        assert!(!key.is_empty(), "OPENROUTER_API_KEY is empty");
        Self {
            key,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("client"),
        }
    }
}

impl PairwiseJudge for OpenRouter {
    fn compare(&self, first: &str, second: &str) -> Verdict {
        let body = json!({
            "model": MODEL,
            "temperature": 0,
            "messages": [
                {"role": "system", "content": SYSTEM},
                {"role": "user", "content": format!(
                    "TASK: Explain why a test should assert on a rendered screen.\n\nA:\n{first}\n\nB:\n{second}"
                )}
            ]
        });
        let response: Value = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&self.key)
            .json(&body)
            .send()
            .expect("request failed")
            .error_for_status()
            .expect("provider returned an error status")
            .json()
            .expect("response was not JSON");
        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_uppercase();
        let side = if content.starts_with('A') {
            Side::First
        } else if content.starts_with('B') {
            Side::Second
        } else {
            Side::Tie
        };
        Verdict::new(side, content)
    }
}

const GOOD: &str =
    "A screen assertion catches escape-sequence bugs that raw byte comparisons miss.";
const BAD: &str = "Bananas are yellow.";

#[test]
#[ignore = "live OpenRouter e2e; requires OPENROUTER_API_KEY"]
fn pairwise_judge_agrees_across_orders_on_a_clear_difference() {
    let outcome = counterbalance(&OpenRouter::new(), GOOD, BAD);
    assert_eq!(
        outcome.agreement,
        Agreement::Consistent,
        "forward={:?} reverse={:?}",
        outcome.forward,
        outcome.reverse
    );
    assert_eq!(outcome.winner(), Some(Side::First));
}

#[test]
#[ignore = "live OpenRouter e2e; requires OPENROUTER_API_KEY"]
fn pairwise_judge_follows_content_when_the_order_is_swapped() {
    let outcome = counterbalance(&OpenRouter::new(), BAD, GOOD);
    assert_eq!(
        outcome.agreement,
        Agreement::Consistent,
        "forward={:?} reverse={:?}",
        outcome.forward,
        outcome.reverse
    );
    assert_eq!(outcome.winner(), Some(Side::Second));
}
