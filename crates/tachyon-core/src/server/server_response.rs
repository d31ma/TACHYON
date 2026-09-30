use super::HeaderNames;
use super::HotUpdateHub;
use axum::Router;
use axum::body::Body;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_TYPE, HeaderValue, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::response::Builder;
use axum::http::{Response, StatusCode};
use tachyon_contracts::{HandlerHeaders, HandlerResponse};
use tower_http::set_header::SetResponseHeaderLayer;

pub(in crate::server) struct ServerResponse;

impl ServerResponse {
    const FRAMING_HEADERS: [&'static str; 3] =
        ["content-length", "transfer-encoding", "connection"];

    pub(in crate::server) fn text(status: StatusCode, message: &str) -> Response<Body> {
        Response::builder()
            .status(status)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Body::from(String::from(message)))
            .unwrap_or_else(|_| Response::new(Body::empty()))
    }

    pub(in crate::server) fn invocation_failure(kind: &str, request_id: &str) -> Response<Body> {
        let mut response = Self::text(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("{kind} failed. Reference: {request_id}."),
        );
        Self::insert_request_id(&mut response, request_id);
        response
    }

    pub(in crate::server) fn insert_request_id(response: &mut Response<Body>, request_id: &str) {
        Self::insert_request_id_value(response, HeaderValue::from_str(request_id).ok());
    }

    pub(in crate::server) fn from_handler(response: HandlerResponse) -> Response<Body> {
        let status =
            StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let has_content_type = Self::has_content_type(&response.headers);
        let builder =
            Self::with_handler_headers(Response::builder().status(status), &response.headers);
        let builder = Self::with_default_content_type(builder, has_content_type);
        let body = response.body.map(|body| body.data).unwrap_or_default();
        builder.body(Body::from(body)).unwrap_or_else(|_| {
            Self::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid handler response.",
            )
        })
    }

    pub(in crate::server) fn from_middleware(response: HandlerResponse) -> Response<Body> {
        let status =
            StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let builder =
            Self::with_handler_headers(Response::builder().status(status), &response.headers);
        let body = response.body.map(|body| body.data).unwrap_or_default();
        builder.body(Body::from(body)).unwrap_or_else(|_| {
            Self::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid middleware response.",
            )
        })
    }

    pub(in crate::server) fn live_script() -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/javascript; charset=utf-8")
            .body(Body::from(HotUpdateHub::LIVE_RELOAD_CLIENT))
            .unwrap_or_else(|_| {
                Self::text(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cannot serve the client.",
                )
            })
    }

    pub(in crate::server) fn is_framing_header(name: &str) -> bool {
        Self::FRAMING_HEADERS
            .iter()
            .any(|framing| name.eq_ignore_ascii_case(framing))
    }

    pub(in crate::server) fn defensive_headers(
        application: Router,
        hosts: &crate::safety::DeclaredHosts,
    ) -> Router {
        application
            .layer(SetResponseHeaderLayer::if_not_present(
                CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                REFERRER_POLICY,
                HeaderValue::from_static("no-referrer"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderNames::X_FRAME_OPTIONS,
                HeaderValue::from_static("DENY"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ))
            .layer(SetResponseHeaderLayer::if_not_present(
                HeaderNames::CONTENT_SECURITY_POLICY,
                Self::content_security_policy(hosts),
            ))
    }

    fn content_security_policy(hosts: &crate::safety::DeclaredHosts) -> HeaderValue {
        let policy = crate::compiler::ContentSecurityPolicy::server(&hosts.connect_sources());
        HeaderValue::from_str(&policy)
            .unwrap_or_else(|_| HeaderValue::from_static("default-src 'self'"))
    }

    fn insert_request_id_value(response: &mut Response<Body>, value: Option<HeaderValue>) {
        if let Some(value) = value {
            response
                .headers_mut()
                .insert(HeaderNames::REQUEST_ID, value);
        }
    }

    fn with_handler_headers(builder: Builder, headers: &HandlerHeaders) -> Builder {
        headers
            .iter()
            .filter(|(name, _)| !Self::is_framing_header(name))
            .flat_map(|(name, values)| values.iter().map(move |value| (name, value)))
            .fold(builder, |builder, (name, value)| {
                builder.header(name, value)
            })
    }

    fn has_content_type(headers: &HandlerHeaders) -> bool {
        headers
            .iter()
            .any(|(name, values)| Self::is_content_type_with_value(name, values))
    }

    fn is_content_type_with_value(name: &str, values: &[String]) -> bool {
        name.eq_ignore_ascii_case("content-type") && !values.is_empty()
    }

    fn with_default_content_type(builder: Builder, has_content_type: bool) -> Builder {
        if has_content_type {
            return builder;
        }
        builder.header(CONTENT_TYPE, "application/json; charset=utf-8")
    }
}
