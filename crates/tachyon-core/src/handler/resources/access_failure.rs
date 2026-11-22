use crate::resource_error_code::ResourceErrorCode;
use crate::sorted_json::SortedJson;
use axum::http::StatusCode;
use serde_json::json;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AccessFailure {
    pub(super) status: StatusCode,
    pub(super) code: ResourceErrorCode,
}

impl AccessFailure {
    const MESSAGE: &'static str = "Yon managed resource operation failed.";

    pub(super) const fn new(status: StatusCode, code: ResourceErrorCode) -> Self {
        Self { status, code }
    }

    pub(super) fn body(self) -> Vec<u8> {
        let answer = json!({ "error": Self::MESSAGE, "code": self.code.text() });
        SortedJson::text(&answer).into_bytes()
    }

    fn code_of_status(status: StatusCode) -> ResourceErrorCode {
        match status {
            StatusCode::REQUEST_TIMEOUT => ResourceErrorCode::BudgetSpent,
            StatusCode::BAD_GATEWAY => ResourceErrorCode::TooLarge,
            _ => ResourceErrorCode::Failed,
        }
    }
}

impl From<StatusCode> for AccessFailure {
    fn from(status: StatusCode) -> Self {
        Self::new(status, Self::code_of_status(status))
    }
}

#[cfg(test)]
mod tests {
    use super::AccessFailure;
    use crate::resource_error_code::ResourceErrorCode;
    use axum::http::StatusCode;

    fn body_of(failure: AccessFailure) -> String {
        String::from_utf8(failure.body()).unwrap_or_default()
    }

    #[test]
    fn the_body_keeps_the_message_and_adds_the_code() {
        let failure = AccessFailure::new(StatusCode::BAD_REQUEST, ResourceErrorCode::Conflict);
        assert_eq!(
            body_of(failure),
            r#"{"code":"conflict","error":"Yon managed resource operation failed."}"#
        );
    }

    #[test]
    fn a_status_alone_names_its_code() {
        let codes = [
            StatusCode::REQUEST_TIMEOUT,
            StatusCode::BAD_GATEWAY,
            StatusCode::FORBIDDEN,
            StatusCode::BAD_REQUEST,
        ]
        .map(|status| AccessFailure::from(status).code);
        assert_eq!(
            codes,
            [
                ResourceErrorCode::BudgetSpent,
                ResourceErrorCode::TooLarge,
                ResourceErrorCode::Failed,
                ResourceErrorCode::Failed,
            ]
        );
    }
}
