use crate::Failure;
use crate::sorted_json::SortedJson;
use serde_json::Value;
use tachyon_contracts::{HandlerBodyEncoding, HandlerResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::server) enum IdentityAnswer {
    NotAsked,
    Nobody,
    SignedIn { roles: Vec<String>, identity: Value },
    Failed,
}

impl IdentityAnswer {
    const NO_CONTENT: u16 = 204;
    const SUCCESS: std::ops::Range<u16> = 200..300;
    const ROLES_FIELD: &'static str = "roles";

    pub(in crate::server) fn read(result: Result<HandlerResponse, Failure>) -> Self {
        result.map_or(Self::Failed, |response| Self::from_response(&response))
    }

    pub(in crate::server) fn into_identity(self) -> Option<Value> {
        self.signed_in_identity()
    }

    fn signed_in_identity(self) -> Option<Value> {
        match self {
            Self::SignedIn { identity, .. } => Some(identity),
            _ => None,
        }
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
            (Value::Object(_), None | Some(Value::Null)) => Self::signed_in(Vec::new(), value),
            (Value::Object(_), Some(Value::Array(items))) => Self::role_names(items, value),
            _ => Self::Failed,
        }
    }

    fn role_names(items: &[Value], value: &Value) -> Self {
        let names: Option<Vec<String>> = items
            .iter()
            .map(|item| item.as_str().map(String::from))
            .collect();
        names.map_or(Self::Failed, |roles| Self::signed_in(roles, value))
    }

    fn signed_in(roles: Vec<String>, value: &Value) -> Self {
        let identity = SortedJson::into_sorted(value.clone());
        Self::SignedIn { roles, identity }
    }
}
