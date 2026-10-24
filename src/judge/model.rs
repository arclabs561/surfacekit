//! Provider-neutral model seam for the judge layer.

use serde_json::Value;

/// The result of one bounded JSON request to a judge model.
#[derive(Debug, Clone, PartialEq)]
pub enum JudgeOutcome {
    Valid(Value),
    ProviderFailure(String),
    InvalidJson(String),
}

impl JudgeOutcome {
    #[must_use]
    pub fn status(&self) -> &'static str {
        match self {
            Self::Valid(_) => "valid",
            Self::ProviderFailure(_) => "provider_failure",
            Self::InvalidJson(_) => "invalid_json",
        }
    }

    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::Valid(_) => "",
            Self::ProviderFailure(detail) | Self::InvalidJson(detail) => detail,
        }
    }

    #[must_use]
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Valid(value) => Some(value),
            _ => None,
        }
    }
}

/// One persona whose framing the judge evaluates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Persona {
    pub name: String,
    pub description: String,
}

impl Persona {
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// A caller-owned transport for one bounded judge request.
///
/// Implementations own endpoint, authentication, retries, and wire envelopes.
/// The judge layer only sees the parsed outcome.
pub trait JudgeModel {
    fn call_json(&self, system: &str, user: &str) -> JudgeOutcome;
}
