use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerViolation {
    pub field: String,
    pub rule: String,
}

impl HandlerViolation {
    #[must_use]
    pub fn new(field: impl Into<String>, rule: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            rule: rule.into(),
        }
    }
}
