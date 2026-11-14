use crate::check_rejection::CheckProblem;
use crate::yon::built_in_error::BuiltInError;
use axum::body::Body;
use axum::http::Response;
use serde_json::{Map, Value, json};
use tachyon_contracts::HandlerViolation;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::server) struct BuiltInCase {
    error: BuiltInError,
    details: Map<String, Value>,
}

impl BuiltInCase {
    const TRUNCATED: &'static str = "truncated";

    pub(in crate::server) fn plain(error: BuiltInError) -> Self {
        Self {
            error,
            details: Map::new(),
        }
    }

    pub(in crate::server) fn request_rejected(part: &str, field: &str, message: &str) -> Self {
        let details = json!({ "part": part, "field": field, "message": message });
        Self::with_details(BuiltInError::REQUEST_REJECTED, details)
    }

    pub(in crate::server) fn body_too_large(limit_bytes: usize) -> Self {
        let details = json!({ "limitBytes": limit_bytes });
        Self::with_details(BuiltInError::BODY_TOO_LARGE, details)
    }

    pub(in crate::server) fn check_rejected(violations: &[HandlerViolation]) -> Self {
        let problem = serde_json::to_value(CheckProblem::new(violations, "")).unwrap_or_default();
        let details = json!({
            "violations": problem["errors"].clone(),
            Self::TRUNCATED: problem[Self::TRUNCATED].clone(),
        });
        Self::with_details(BuiltInError::CHECK_REJECTED, details)
    }

    fn with_details(error: BuiltInError, details: Value) -> Self {
        let Value::Object(details) = details else {
            return Self::plain(error);
        };
        Self { error, details }
    }

    pub(in crate::server) const fn error(&self) -> BuiltInError {
        self.error
    }

    pub(in crate::server) fn tag(self, response: Response<Body>) -> Response<Body> {
        Self::tag_option(Some(self), response)
    }

    pub(in crate::server) fn tag_option(
        case: Option<Self>,
        mut response: Response<Body>,
    ) -> Response<Body> {
        response.extensions_mut().insert(case);
        response
    }

    pub(in crate::server) fn of(response: &Response<Body>) -> Option<Self> {
        response
            .extensions()
            .get::<Option<Self>>()
            .cloned()
            .flatten()
    }

    pub(in crate::server) fn argument(&self, reference: &str) -> Value {
        let mut argument = self.details.clone();
        argument.insert(String::from("type"), json!(self.error.name()));
        argument.insert(String::from("status"), json!(self.error.status()));
        argument.insert(String::from("reference"), json!(reference));
        Value::Object(argument)
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltInCase;
    use crate::yon::built_in_error::BuiltInError;
    use axum::body::Body;
    use axum::http::Response;
    use tachyon_contracts::HandlerViolation;

    #[test]
    fn a_plain_case_carries_its_type_status_and_reference() {
        let case = BuiltInCase::plain(BuiltInError::NOT_SIGNED_IN);
        assert_eq!(
            case.argument("ref1").to_string(),
            r#"{"type":"NotSignedIn","status":401,"reference":"ref1"}"#
        );
    }

    #[test]
    fn a_check_rejection_lists_sorted_broken_rules_without_values() {
        let violations = [
            HandlerViolation::new("name", "NotEmpty"),
            HandlerViolation::new("level", "Min"),
        ];
        let argument = BuiltInCase::check_rejected(&violations).argument("r");
        assert_eq!(
            argument["violations"].to_string(),
            r#"[{"field":"level","rule":"Min"},{"field":"name","rule":"NotEmpty"}]"#
        );
        assert_eq!(argument["truncated"], false);
        assert_eq!(argument["status"], 422);
    }

    #[test]
    fn a_rejected_request_and_a_large_body_keep_only_safe_details() {
        let rejected = BuiltInCase::request_rejected(
            "query",
            "page",
            "The query value 'page' must be an integer.",
        );
        assert_eq!(rejected.argument("r")["part"], "query");
        assert_eq!(rejected.argument("r")["field"], "page");
        let large = BuiltInCase::body_too_large(1024);
        assert_eq!(large.argument("r")["limitBytes"], 1024);
        assert_eq!(large.argument("r")["status"], 413);
    }

    #[test]
    fn a_tag_travels_with_the_response() {
        let response =
            BuiltInCase::plain(BuiltInError::ROLE_MISSING).tag(Response::new(Body::empty()));
        assert_eq!(
            BuiltInCase::of(&response).map(|case| case.error()),
            Some(BuiltInError::ROLE_MISSING)
        );
        assert_eq!(BuiltInCase::of(&Response::new(Body::empty())), None);
    }
}
