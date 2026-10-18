use crate::safety::AppOrigins;
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    ACCESS_CONTROL_MAX_AGE, ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
    VARY,
};
use axum::http::{HeaderMap, HeaderValue, Method, Response, StatusCode};
use axum::middleware::Next;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(in crate::server) struct CrossOrigin {
    origins: Arc<AppOrigins>,
}

impl CrossOrigin {
    const METHODS: &'static str = "GET, HEAD, POST, PUT, PATCH, DELETE";
    const MAX_AGE_SECONDS: &'static str = "600";
    const VARY_ORIGIN: &'static str = "Origin";
    const HEADER_SEPARATOR: char = ',';
    const MAX_REQUESTED_HEADERS: usize = 64;

    pub(in crate::server) fn new(origins: &AppOrigins) -> Self {
        Self {
            origins: Arc::new(origins.clone()),
        }
    }

    pub(in crate::server) fn protect(self, application: Router) -> Router {
        application.layer(axum::middleware::from_fn_with_state(self, Self::answer))
    }

    async fn answer(State(cross): State<Self>, request: Request, next: Next) -> Response<Body> {
        let origin = cross.allowed_origin(request.headers());
        let preflight = origin
            .as_ref()
            .is_some_and(|_| Self::is_preflight(&request));
        let mut response = Self::respond(preflight, request, next).await;
        cross.allow(response.headers_mut(), origin);
        response
    }

    async fn respond(preflight: bool, request: Request, next: Next) -> Response<Body> {
        if preflight {
            return Self::preflight(request.headers());
        }
        next.run(request).await
    }

    fn allowed_origin(&self, headers: &HeaderMap) -> Option<HeaderValue> {
        let origin = headers.get(ORIGIN)?;
        let text = origin.to_str().ok()?;
        self.origins.allows(text).then(|| origin.clone())
    }

    fn is_preflight(request: &Request) -> bool {
        let options = request.method() == Method::OPTIONS;
        options
            && request
                .headers()
                .contains_key(ACCESS_CONTROL_REQUEST_METHOD)
    }

    fn preflight(headers: &HeaderMap) -> Response<Body> {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NO_CONTENT;
        let answer = response.headers_mut();
        answer.insert(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(Self::METHODS),
        );
        answer.insert(
            ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static(Self::MAX_AGE_SECONDS),
        );
        answer.extend(
            Self::requested_headers(headers).map(|names| (ACCESS_CONTROL_ALLOW_HEADERS, names)),
        );
        response
    }

    fn requested_headers(headers: &HeaderMap) -> Option<HeaderValue> {
        let requested = headers.get(ACCESS_CONTROL_REQUEST_HEADERS)?;
        let names: Vec<&str> = requested
            .to_str()
            .ok()?
            .split(Self::HEADER_SEPARATOR)
            .map(str::trim)
            .collect();
        let readable = names.iter().all(|name| Self::is_header_name(name));
        let few = names.len() <= Self::MAX_REQUESTED_HEADERS;
        (readable && few).then(|| requested.clone())
    }

    fn is_header_name(name: &str) -> bool {
        axum::http::HeaderName::from_bytes(name.as_bytes()).is_ok()
    }

    fn allow(&self, headers: &mut HeaderMap, origin: Option<HeaderValue>) {
        let listed = !self.origins.is_empty();
        headers.extend(listed.then(|| (VARY, HeaderValue::from_static(Self::VARY_ORIGIN))));
        if let Some(origin) = origin {
            headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        }
    }
}
