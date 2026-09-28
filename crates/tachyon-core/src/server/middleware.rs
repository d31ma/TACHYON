use super::{Dispatch, InvocationLog, MiddlewareOutcome, ProtocolRequest, ServerResponse};
use crate::Failure;
use crate::handler::HandlerSource;
use axum::body::Body;
use axum::http::header::{HeaderName, HeaderValue};
use axum::http::{HeaderMap, Response, Uri};
use tachyon_contracts::{HandlerRequest, HandlerResponse, HttpMethod};

pub(in crate::server) struct Middleware;

impl Middleware {
    pub(in crate::server) async fn before(
        state: &Dispatch,
        middleware: &HandlerSource,
        uri: &Uri,
        method: HttpMethod,
        headers: &HeaderMap,
    ) -> MiddlewareOutcome {
        let mut protocol_request = Self::request(uri, method);
        protocol_request.operation = String::from("middleware.before");
        ProtocolRequest::copy_headers(&mut protocol_request, headers);
        let result = state.invoke(middleware, &protocol_request).await;
        Self::before_outcome(result, &protocol_request, middleware)
    }

    pub(in crate::server) async fn after(
        state: &Dispatch,
        middleware: &HandlerSource,
        uri: &Uri,
        method: HttpMethod,
        request_headers: &HeaderMap,
        response: Response<Body>,
    ) -> Response<Body> {
        let mut protocol_request = Self::request(uri, method);
        protocol_request.operation = String::from("middleware.after");
        ProtocolRequest::copy_headers(&mut protocol_request, request_headers);
        Self::copy_response_headers(&mut protocol_request, &response);
        let result = state.invoke(middleware, &protocol_request).await;
        Self::after_response(result, &protocol_request, middleware, response)
    }

    fn request(uri: &Uri, method: HttpMethod) -> HandlerRequest {
        let mut request = HandlerRequest::route(crate::ttid::generate(), uri.path(), method);
        request.target = ProtocolRequest::target(uri);
        request
    }

    fn before_outcome(
        result: Result<HandlerResponse, Failure>,
        protocol_request: &HandlerRequest,
        middleware: &HandlerSource,
    ) -> MiddlewareOutcome {
        match result {
            Ok(response) if response.status == 204 => MiddlewareOutcome::Continue,
            Ok(response) => MiddlewareOutcome::Respond(ServerResponse::from_middleware(response)),
            Err(failure) => {
                InvocationLog::log_failure(
                    "middleware",
                    &protocol_request.request_id,
                    InvocationLog::runtime_family(middleware),
                    &failure,
                );
                MiddlewareOutcome::Respond(ServerResponse::invocation_failure(
                    "Middleware execution",
                    &protocol_request.request_id,
                ))
            }
        }
    }

    fn after_response(
        result: Result<HandlerResponse, Failure>,
        protocol_request: &HandlerRequest,
        middleware: &HandlerSource,
        response: Response<Body>,
    ) -> Response<Body> {
        match result {
            Ok(decision) if decision.status != 204 => ServerResponse::from_handler(decision),
            Ok(decision) => Self::add_headers(response, &decision),
            Err(failure) => {
                InvocationLog::log_failure(
                    "middleware.after",
                    &protocol_request.request_id,
                    InvocationLog::runtime_family(middleware),
                    &failure,
                );
                response
            }
        }
    }

    fn copy_response_headers(request: &mut HandlerRequest, response: &Response<Body>) {
        request.headers.insert(
            String::from("x-tachyon-status"),
            vec![response.status().as_u16().to_string()],
        );
        response
            .headers()
            .iter()
            .filter_map(|(name, value)| value.to_str().ok().map(|value| (name, value)))
            .for_each(|(name, value)| {
                request
                    .headers
                    .entry(format!("x-tachyon-response-{name}"))
                    .or_default()
                    .push(value.to_owned());
            });
    }

    fn add_headers(response: Response<Body>, decision: &HandlerResponse) -> Response<Body> {
        let (mut parts, body) = response.into_parts();
        decision
            .headers
            .iter()
            .filter(|(name, _)| !ServerResponse::is_framing_header(name))
            .filter_map(|(name, values)| {
                HeaderName::try_from(name.as_str())
                    .ok()
                    .map(|name| (name, values))
            })
            .flat_map(|(name, values)| {
                values.iter().filter_map(move |value| {
                    HeaderValue::from_str(value)
                        .ok()
                        .map(|value| (name.clone(), value))
                })
            })
            .for_each(|(name, value)| {
                parts.headers.insert(name, value);
            });
        Response::from_parts(parts, body)
    }
}
