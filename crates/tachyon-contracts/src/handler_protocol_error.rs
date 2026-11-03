use crate::HandlerViolation;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerProtocolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<HandlerViolation>,
}

impl HandlerProtocolError {
    pub const CHECK_REJECTED: &'static str = "TY2205";
    pub const CHECK_REJECTED_MESSAGE: &'static str = "Input breaks a Checked rule.";

    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            violations: Vec::new(),
        }
    }

    #[must_use]
    pub fn check_rejected(violations: Vec<HandlerViolation>) -> Self {
        Self {
            violations,
            ..Self::new(Self::CHECK_REJECTED, Self::CHECK_REJECTED_MESSAGE)
        }
    }

    #[must_use]
    pub fn is_check_rejection(&self) -> bool {
        self.code == Self::CHECK_REJECTED
    }
}
