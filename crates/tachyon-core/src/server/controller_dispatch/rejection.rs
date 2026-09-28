use super::RouteAnswer;
use axum::body::Body;
use axum::http::{Response, StatusCode};

pub(in crate::server) struct Rejection {
    part: &'static str,
    field: String,
    message: String,
}

impl Rejection {
    pub(super) fn new(part: &'static str, field: &str, message: &str) -> Self {
        Self {
            part,
            field: field.to_owned(),
            message: message.to_owned(),
        }
    }

    pub(in crate::server) fn response(&self) -> Response<Body> {
        let body =
            serde_json::json!({"error": self.message, "part": self.part, "field": self.field});
        RouteAnswer::json(StatusCode::BAD_REQUEST, &body, None)
    }
}
