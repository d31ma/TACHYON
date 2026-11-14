use super::RouteAnswer;
use crate::server::BuiltInCase;
use crate::sorted_json::SortedJson;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response, StatusCode};
use tachyon_contracts::{HandlerBody, HandlerBodyEncoding, HandlerHeaders, HandlerResponse};

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
        let response = RouteAnswer::json(StatusCode::BAD_REQUEST, &self.body(), None);
        BuiltInCase::request_rejected(self.part, &self.field, &self.message).tag(response)
    }

    pub(in crate::server) fn handler_response(&self, request_id: &str) -> HandlerResponse {
        let headers = HandlerHeaders::from([(
            CONTENT_TYPE.as_str().to_owned(),
            vec![RouteAnswer::JSON_CONTENT_TYPE.to_owned()],
        )]);
        let body = HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: SortedJson::text(&self.body()),
        };
        HandlerResponse::success(request_id, StatusCode::BAD_REQUEST.as_u16(), headers, body)
    }

    fn body(&self) -> serde_json::Value {
        serde_json::json!({"error": self.message, "part": self.part, "field": self.field})
    }
}
