use crate::Failure;
use serde_json::Value;
use tachyon_contracts::{HandlerBodyEncoding, HandlerResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::server) enum IdentityAnswer {
    NotAsked,
    Nobody,
    Roles(Vec<String>),
    Failed,
}

impl IdentityAnswer {
    const NO_CONTENT: u16 = 204;
    const SUCCESS: std::ops::Range<u16> = 200..300;
    const ROLES_FIELD: &'static str = "roles";

    pub(in crate::server) fn read(result: Result<HandlerResponse, Failure>) -> Self {
        result.map_or(Self::Failed, |response| Self::from_response(&response))
    }

    fn from_response(response: &HandlerResponse) -> Self {
        let text = Self::successful_text(response);
        text.map_or(Self::Failed, |text| Self::from_text(&text))
    }

    fn successful_text(response: &HandlerResponse) -> Option<String> {
        if !Self::SUCCESS.contains(&response.status) {
            return None;
        }
        Some(Self::body_text(response))
    }

    fn body_text(response: &HandlerResponse) -> String {
        let utf8 = response
            .body
            .as_ref()
            .filter(|body| body.encoding == HandlerBodyEncoding::Utf8);
        let text = utf8.map_or("", |body| body.data.as_str());
        Self::text_for_status(response.status, text)
    }

    fn text_for_status(status: u16, text: &str) -> String {
        if status == Self::NO_CONTENT {
            return String::new();
        }
        String::from(text.trim())
    }

    fn from_text(text: &str) -> Self {
        let value = Self::json(text);
        value.map_or(Self::Failed, |value| Self::from_value(&value))
    }

    fn json(text: &str) -> Option<Value> {
        if text.is_empty() {
            return Some(Value::Null);
        }
        serde_json::from_str(text).ok()
    }

    fn from_value(value: &Value) -> Self {
        match (value, value.get(Self::ROLES_FIELD)) {
            (Value::Null, _) => Self::Nobody,
            (Value::Object(_), None | Some(Value::Null)) => Self::Roles(Vec::new()),
            (Value::Object(_), Some(Value::Array(items))) => Self::role_names(items),
            _ => Self::Failed,
        }
    }

    fn role_names(items: &[Value]) -> Self {
        let names: Option<Vec<String>> = items
            .iter()
            .map(|item| item.as_str().map(String::from))
            .collect();
        names.map_or(Self::Failed, Self::Roles)
    }
}
