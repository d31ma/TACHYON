use serde_json::{Number, Value};

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
    const DOLLAR: char = '$';
    const ESCAPED_DOLLAR: &'static str = "\\u0024";
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

    fn jvm_literal(value: &Value) -> String {
        match value {
            Value::String(_) => Self::jvm_text(value),
            Value::Number(number) => Self::jvm_number(number),
            other => other.to_string(),
        }
    }

    fn jvm_text(value: &Value) -> String {
        value
            .to_string()
            .replace(Self::DOLLAR, Self::ESCAPED_DOLLAR)
    }

    fn jvm_number(number: &Number) -> String {
        let beyond_int = number
            .as_i64()
            .is_some_and(|whole| i32::try_from(whole).is_err());
        if beyond_int {
            return format!("{number}{}", Self::LONG_MARK);
        }
        number.to_string()
    }
}
