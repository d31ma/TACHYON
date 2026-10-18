use super::ServerResponse;
use crate::check_rejection::CheckProblem;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response, StatusCode};
use tachyon_contracts::{HandlerProtocolError, HandlerResponse};

pub(in crate::server) struct CheckAnswer;

impl CheckAnswer {
    pub(in crate::server) fn rejection(
        response: &HandlerResponse,
    ) -> Option<&HandlerProtocolError> {
        response
            .error
            .as_ref()
            .filter(|error| error.is_check_rejection())
    }

    pub(in crate::server) fn response(
        error: &HandlerProtocolError,
        request_id: &str,
    ) -> Response<Body> {
        let problem = CheckProblem::new(&error.violations, request_id);
        let mut response = Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .header(CONTENT_TYPE, CheckProblem::CONTENT_TYPE)
            .body(Body::from(problem.json()))
            .unwrap_or_else(|_| Response::new(Body::empty()));
        ServerResponse::insert_request_id(&mut response, request_id);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::CheckAnswer;
    use axum::http::StatusCode;
    use tachyon_contracts::{HandlerProtocolError, HandlerResponse, HandlerViolation};

    #[tokio::test]
    async fn a_check_rejection_answers_422_problem_details() -> Result<(), axum::Error> {
        let error = HandlerProtocolError::check_rejected(vec![
            HandlerViolation::new("level", "Max"),
            HandlerViolation::new("name", "NotEmpty"),
        ]);
        let handler_response = HandlerResponse::error("ref7", 422, error);
        let Some(found) = CheckAnswer::rejection(&handler_response) else {
            return Err(axum::Error::new("no rejection"));
        };
        let response = CheckAnswer::response(found, "ref7");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        assert_eq!(response.headers()["x-tachyon-request-id"], "ref7");
        let bytes = axum::body::to_bytes(response.into_body(), 4096).await?;
        assert_eq!(
            bytes.as_ref(),
            br#"{"type":"about:blank","title":"Invalid input","status":422,"errors":[{"field":"level","rule":"Max"},{"field":"name","rule":"NotEmpty"}],"truncated":false,"reference":"ref7"}"#
        );
        Ok(())
    }

    #[test]
    fn other_handler_errors_are_not_rejections() {
        let failed = HandlerResponse::error("ref8", 500, HandlerProtocolError::new("TY2201", "x"));
        assert!(CheckAnswer::rejection(&failed).is_none());
    }
}
