use serde_json::Value;

pub(super) struct CallStyle {
    receiver: &'static str,
    pub(super) literal: fn(&Value) -> String,
}

impl CallStyle {
    pub(super) const RUST: Self = Self {
        receiver: "self.",
        literal: Self::rust_literal,
    };
    pub(super) const JVM: Self = Self {
        receiver: "",
        literal: Self::jvm_literal,
    };
    pub(super) const JSON: Self = Self {
        receiver: "",
        literal: Self::json_literal,
    };
    pub(super) const DART: Self = Self {
        receiver: "",
        literal: Self::dart_literal,
    };
    pub(super) const SWIFT: Self = Self {
        receiver: "",
        literal: Self::rust_literal,
    };
    const DOLLAR: char = '$';
    const ESCAPED_DOLLAR: &'static str = "\\u0024";
    const DART_DOLLAR: &'static str = "\\$";
    const LONG_MARK: &'static str = "L";

    pub(super) fn call(&self, method: &str, argument: &str) -> String {
        format!("{}{method}({argument})", self.receiver)
    }

    fn rust_literal(value: &Value) -> String {
        match value {
            Value::String(text) => format!("{text:?}"),
            other => other.to_string(),
        }
    }

    fn json_literal(value: &Value) -> String {
        value.to_string()
    }

    fn dart_literal(value: &Value) -> String {
        value.to_string().replace(Self::DOLLAR, Self::DART_DOLLAR)
    }

    fn jvm_literal(value: &Value) -> String {
        let text = value
            .to_string()
            .replace(Self::DOLLAR, Self::ESCAPED_DOLLAR);
        format!("{text}{}", Self::long_mark(value))
    }

    fn long_mark(value: &Value) -> &'static str {
        let beyond_int = value
            .as_i64()
            .is_some_and(|whole| i32::try_from(whole).is_err());
        if beyond_int {
            return Self::LONG_MARK;
        }
        ""
    }
}
