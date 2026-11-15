use super::RouteAnswer;
use crate::server::{BuiltInCase, IdentityMismatch};
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
    const IDENTITY_PART: &'static str = "identity";
    const IDENTITY_MISMATCH: &'static str = "This route needs the identity of the signed-in caller, and that identity is missing or does not fit its model.";

    pub(super) fn new(part: &'static str, field: &str, message: &str) -> Self {
        Self {
            part,
            field: field.to_owned(),
            message: message.to_owned(),
        }
    }

    pub(super) fn identity_mismatch() -> Self {
        Self::new(Self::IDENTITY_PART, "", Self::IDENTITY_MISMATCH)
    }

    pub(in crate::server) fn response(&self, route: &str) -> Response<Body> {
        self.response_for(route)
    }

    fn response_for(&self, route: &str) -> Response<Body> {
        if self.part == Self::IDENTITY_PART {
            return IdentityMismatch::response(route);
        }
        self.bad_request()
    }

    fn bad_request(&self) -> Response<Body> {
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
