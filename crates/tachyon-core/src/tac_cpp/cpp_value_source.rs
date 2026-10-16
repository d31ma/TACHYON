use super::{CppAttribute, CppFailure};
use crate::Failure;
use serde_json::Value;
use std::path::Path;

pub(crate) struct CppValueSource;

impl CppValueSource {
    const MOST_INPUTS: usize = 256;
    const NOT_A_LIST: &'static str = "ValueSource takes one list of 1 to 256 literal inputs, for example [[ValueSource([1, 2, 3])]]";
    const NOT_PLAIN: &'static str = "In C++, ValueSource lists numbers, strings or booleans, for example [[ValueSource([1, 2, 3])]]";

    pub(crate) fn inputs(relative: &Path, marker: &CppAttribute) -> Result<Vec<String>, Failure> {
        let values = Self::values(relative, marker)?;
        values
            .iter()
            .map(Self::literal)
            .collect::<Option<Vec<String>>>()
            .ok_or_else(|| CppFailure::unreadable(relative, marker.at, Self::NOT_PLAIN))
    }

    fn values(relative: &Path, marker: &CppAttribute) -> Result<Vec<Value>, Failure> {
        let parsed = serde_json::from_str::<Value>(&marker.arguments).ok();
        let Some(Value::Array(values)) = parsed else {
            return Err(CppFailure::unreadable(
                relative,
                marker.at,
                Self::NOT_A_LIST,
            ));
        };
        let counted = (1..=Self::MOST_INPUTS).contains(&values.len());
        counted
            .then_some(values)
            .ok_or_else(|| CppFailure::unreadable(relative, marker.at, Self::NOT_A_LIST))
    }

    fn literal(value: &Value) -> Option<String> {
        match value {
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(flag) => Some(flag.to_string()),
            Value::String(text) => Some(Self::quoted(text)),
            _ => None,
        }
    }

    fn quoted(text: &str) -> String {
        let mut written = String::from("\"");
        for character in text.chars() {
            written.push_str(&Self::escaped(character));
        }
        written.push('"');
        written
    }

    fn escaped(character: char) -> String {
        match character {
            '\\' => String::from("\\\\"),
            '"' => String::from("\\\""),
            control if control.is_ascii_control() => format!("\\{:03o}", u32::from(control)),
            plain => plain.to_string(),
        }
    }
}
