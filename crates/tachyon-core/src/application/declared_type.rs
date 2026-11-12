use super::{Failure, invalid};
use serde_json::{Number, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DeclaredType {
    Text,
    Boolean,
    WholeNumber,
    Number,
}

impl DeclaredType {
    const POINTER_SEPARATOR: &'static str = "/";
    const SETTING_SEPARATOR: &'static str = ".";

    pub(super) fn typed_value(
        document: &Value,
        name: &str,
        path: &[String],
        text: String,
    ) -> Result<Value, Failure> {
        let declared = Self::declared(document, name, path)?;
        let converted = declared.converted(text);
        converted.ok_or_else(|| invalid(declared.mismatch(name, path)))
    }

    fn declared(document: &Value, name: &str, path: &[String]) -> Result<Self, Failure> {
        let pointer = Self::POINTER_SEPARATOR.to_owned() + &path.join(Self::POINTER_SEPARATOR);
        let value = document.pointer(&pointer);
        Self::of(value).ok_or_else(|| invalid(Self::not_replaceable(name, path)))
    }

    fn of(value: Option<&Value>) -> Option<Self> {
        match (value, value.is_some_and(Value::is_f64)) {
            (None | Some(Value::Null | Value::String(_)), _) => Some(Self::Text),
            (Some(Value::Bool(_)), _) => Some(Self::Boolean),
            (Some(Value::Number(_)), false) => Some(Self::WholeNumber),
            (Some(Value::Number(_)), true) => Some(Self::Number),
            (Some(Value::Array(_) | Value::Object(_)), _) => None,
        }
    }

    fn converted(self, text: String) -> Option<Value> {
        match self {
            Self::Text => Some(Value::String(text)),
            Self::Boolean => text.parse::<bool>().ok().map(Value::Bool),
            Self::WholeNumber => Self::number(&text).filter(Self::is_whole),
            Self::Number => Self::number(&text),
        }
    }

    fn number(text: &str) -> Option<Value> {
        serde_json::from_str::<Number>(text).ok().map(Value::Number)
    }

    fn is_whole(value: &Value) -> bool {
        !value.is_f64()
    }

    fn mismatch(self, name: &str, path: &[String]) -> String {
        let expected = match self {
            Self::Text => "text",
            Self::Boolean => "true or false",
            Self::WholeNumber => "a whole number",
            Self::Number => "a number",
        };
        let setting = path.join(Self::SETTING_SEPARATOR);
        format!(
            "{name} in .env must be {expected}, because {setting} is {expected} in server/application.json or the selected profile."
        )
    }

    fn not_replaceable(name: &str, path: &[String]) -> String {
        let setting = path.join(Self::SETTING_SEPARATOR);
        format!(
            "{name} in .env cannot replace {setting}, because {setting} is a list or an object. .env can replace only a text, true or false, or number setting; write lists and objects in server/application.json."
        )
    }
}
