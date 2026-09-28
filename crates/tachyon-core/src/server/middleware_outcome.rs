use axum::body::Body;
use axum::http::Response;

pub(in crate::server) enum MiddlewareOutcome {
    Continue,
    Respond(Response<Body>),
}
