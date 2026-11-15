use super::{BuiltInCase, ServerResponse};
use crate::sorted_json::SortedJson;
use crate::yon::built_in_error::BuiltInError;
use axum::body::Body;
use axum::http::{Response, StatusCode};

pub(in crate::server) struct IdentityMismatch;

impl IdentityMismatch {
    const EVENT: &'static str = "route.identity_mismatch";

    pub(in crate::server) fn response(route: &str) -> Response<Body> {
        let request_id = crate::ttid::generate();
        eprintln!("{}", Self::event(&request_id, route));
        let message = format!("Handler execution failed. Reference: {request_id}.");
        let mut response = ServerResponse::text(StatusCode::INTERNAL_SERVER_ERROR, &message);
        ServerResponse::insert_request_id(&mut response, &request_id);
        BuiltInCase::plain(BuiltInError::UNEXPECTED_ERROR).tag(response)
    }

    fn event(request_id: &str, route: &str) -> String {
        let event = serde_json::json!({
            "event": Self::EVENT,
            "request_id": request_id,
            "route": route,
        });
        SortedJson::text(&event)
    }
}

#[cfg(test)]
mod tests {
    use super::IdentityMismatch;
    use crate::server::BuiltInCase;
    use crate::yon::built_in_error::BuiltInError;
    use axum::http::StatusCode;

    #[test]
    fn the_log_line_names_the_request_and_the_route_only() {
        let line = IdentityMismatch::event("r-1", "/me");
        assert_eq!(
            line,
            r#"{"event":"route.identity_mismatch","request_id":"r-1","route":"/me"}"#
        );
    }

    #[test]
    fn the_answer_is_the_fixed_server_error_tagged_as_unexpected() {
        let response = IdentityMismatch::response("/me");
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().contains_key("x-tachyon-request-id"));
        let case = BuiltInCase::of(&response).map(|case| case.error());
        assert_eq!(case, Some(BuiltInError::UNEXPECTED_ERROR));
    }
}
