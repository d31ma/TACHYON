use super::RouteAnswer;
use crate::route_types::{ModelTable, Shape};
use crate::yon::plan::{ErrorResponse, Route};
use axum::body::Body;
use axum::http::{Response, StatusCode};
use serde_json::Value;
use tachyon_contracts::HandlerResponse;

pub(in crate::server) struct RouteResult;

impl RouteResult {
    pub(in crate::server) fn problem(
        route: &Route,
        models: &ModelTable,
        response: &HandlerResponse,
    ) -> Option<String> {
        let success = (200..300).contains(&response.status);
        Self::problem_by_status(success, route, models, response)
    }

    pub(in crate::server) fn mismatch(
        route: &Route,
        problem: &str,
        request_id: &str,
    ) -> Response<Body> {
        let event = serde_json::json!({
            "event": "route.answer_mismatch",
            "request_id": request_id,
            "method": route.method,
            "problem": problem,
        });
        eprintln!("{event}");
        let body = serde_json::json!({
            "error": "The answer does not match the declared return type.",
            "reference": request_id,
        });
        RouteAnswer::json(StatusCode::INTERNAL_SERVER_ERROR, &body, None)
    }

    fn problem_by_status(
        success: bool,
        route: &Route,
        models: &ModelTable,
        response: &HandlerResponse,
    ) -> Option<String> {
        if success {
            return Self::success_problem(route, models, response);
        }
        Self::error_problem(route, models, response)
    }

    fn success_problem(
        route: &Route,
        models: &ModelTable,
        response: &HandlerResponse,
    ) -> Option<String> {
        Self::checks_answer(route, response)
            .then(|| Self::answer_problem(route, models, response))
            .flatten()
    }

    fn checks_answer(route: &Route, response: &HandlerResponse) -> bool {
        route.return_shape != Shape::Any && Self::is_json(response)
    }

    fn answer_problem(
        route: &Route,
        models: &ModelTable,
        response: &HandlerResponse,
    ) -> Option<String> {
        serde_json::from_str::<Value>(Self::body_text(response)).map_or_else(
            |_| Some(String::from("the answer is not valid JSON")),
            |value| models.check(&route.return_shape, &value, "answer").err(),
        )
    }

    fn error_problem(
        route: &Route,
        models: &ModelTable,
        response: &HandlerResponse,
    ) -> Option<String> {
        let errors: Vec<&ErrorResponse> = route
            .error_responses
            .iter()
            .filter(|error| error.status == response.status)
            .collect();
        let declared = !errors.is_empty();
        declared
            .then(|| Self::error_answer_problem(&errors, models, response))
            .flatten()
    }

    fn error_answer_problem(
        errors: &[&ErrorResponse],
        models: &ModelTable,
        response: &HandlerResponse,
    ) -> Option<String> {
        serde_json::from_str::<Value>(Self::body_text(response)).map_or_else(
            |_| Some(String::from("the error answer is not valid JSON")),
            |value| Self::unmatched_error(errors, models, &value),
        )
    }

    fn unmatched_error(
        errors: &[&ErrorResponse],
        models: &ModelTable,
        value: &Value,
    ) -> Option<String> {
        if errors
            .iter()
            .any(|error| models.check(&error.return_shape, value, "answer").is_ok())
        {
            return None;
        }
        Some(String::from(
            "the error answer does not match a declared error return type",
        ))
    }

    fn body_text(response: &HandlerResponse) -> &str {
        response.body.as_ref().map_or("", |body| body.data.as_str())
    }

    fn is_json(response: &HandlerResponse) -> bool {
        response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .is_none_or(|(_, values)| values.iter().any(|value| value.contains("json")))
    }
}
