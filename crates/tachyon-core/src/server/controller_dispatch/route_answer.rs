use crate::routing::RouteMethod;
use crate::yon::plan::Route;
use axum::body::Body;
use axum::http::header::{ALLOW, CONTENT_TYPE};
use axum::http::response::Builder;
use axum::http::{Response, StatusCode};
use tachyon_contracts::HttpMethod;

pub(in crate::server) struct RouteAnswer;

impl RouteAnswer {
    pub(in crate::server) const JSON_CONTENT_TYPE: &'static str = "application/json; charset=utf-8";

    pub(in crate::server) fn method_not_allowed(verbs: &[HttpMethod]) -> Response<Body> {
        let body = serde_json::json!({
            "error": "This path does not answer this HTTP method.",
            "allow": Self::allow(verbs),
        });
        Self::json(StatusCode::METHOD_NOT_ALLOWED, &body, Some(verbs))
    }

    pub(in crate::server) fn options(
        verbs: &[HttpMethod],
        schema: &serde_json::Value,
    ) -> Response<Body> {
        Self::json(StatusCode::OK, schema, Some(verbs))
    }

    pub(in crate::server) fn with_status(
        mut response: Response<Body>,
        route: &Route,
    ) -> Response<Body> {
        let current = response.status();
        let status = route
            .status
            .filter(|_| current == StatusCode::OK)
            .and_then(|status| StatusCode::from_u16(status).ok());
        Self::replace_status(&mut response, status);
        response
    }

    pub(in crate::server) fn json(
        status: StatusCode,
        body: &serde_json::Value,
        verbs: Option<&[HttpMethod]>,
    ) -> Response<Body> {
        let builder = Response::builder()
            .status(status)
            .header(CONTENT_TYPE, Self::JSON_CONTENT_TYPE);
        Self::with_allow(builder, verbs)
            .body(Body::from(body.to_string()))
            .unwrap_or_else(|_| {
                let mut fallback = Response::new(Body::empty());
                *fallback.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                fallback
            })
    }

    fn replace_status(response: &mut Response<Body>, status: Option<StatusCode>) {
        if let Some(status) = status {
            *response.status_mut() = status;
        }
    }

    fn with_allow(builder: Builder, verbs: Option<&[HttpMethod]>) -> Builder {
        if let Some(verbs) = verbs {
            return builder.header(ALLOW, Self::allow(verbs));
        }
        builder
    }

    fn allow(verbs: &[HttpMethod]) -> String {
        verbs
            .iter()
            .map(|verb| RouteMethod::name(*verb))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
