use super::HeaderNames;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response};
use axum::middleware::Next;
use std::time::{Duration, Instant};

pub(in crate::server) struct AccessLog;

impl AccessLog {
    const SETTING: &'static str = "TAC_ACCESS_LOG";

    pub(in crate::server) fn wrap(router: Router) -> Router {
        let enabled = crate::setting_switch::SettingSwitch::is_set(Self::SETTING);
        Self::wrap_when(router, enabled)
    }

    fn wrap_when(router: Router, enabled: bool) -> Router {
        if !enabled {
            return router;
        }
        router.layer(axum::middleware::from_fn(Self::record))
    }

    pub(in crate::server) fn event(
        method: &str,
        path: &str,
        response: &Response<Body>,
        elapsed: Duration,
    ) -> String {
        let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let mut event = serde_json::json!({
            "event": "http.access",
            "method": method,
            "path": path,
            "status": response.status().as_u16(),
            "duration_ms": duration_ms,
        });
        Self::add_request_id(&mut event, response);
        event.to_string()
    }

    fn add_request_id(event: &mut serde_json::Value, response: &Response<Body>) {
        if let Some(request_id) = response
            .headers()
            .get(HeaderNames::REQUEST_ID)
            .and_then(|value| value.to_str().ok())
        {
            event["request_id"] = serde_json::json!(request_id);
        }
    }

    async fn record(request: Request<Body>, next: Next) -> Response<Body> {
        let started = Instant::now();
        let method = request.method().to_string();
        let path = request.uri().path().to_owned();
        let response = next.run(request).await;
        eprintln!(
            "{}",
            Self::event(&method, &path, &response, started.elapsed())
        );
        response
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AccessLog;
    use crate::server::HeaderNames;
    use axum::body::Body;
    use axum::http::{HeaderValue, Response, StatusCode};
    use std::time::Duration;

    #[test]
    fn an_access_event_names_the_request_status_and_reference() {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::GATEWAY_TIMEOUT;
        response
            .headers_mut()
            .insert(HeaderNames::REQUEST_ID, HeaderValue::from_static("0ABC"));
        let event = AccessLog::event("GET", "/items/7", &response, Duration::from_millis(42));
        let event: serde_json::Value = serde_json::from_str(&event).expect("JSON event");
        assert_eq!(event["event"], "http.access");
        assert_eq!(event["method"], "GET");
        assert_eq!(event["path"], "/items/7");
        assert_eq!(event["status"], 504);
        assert_eq!(event["duration_ms"], 42);
        assert_eq!(event["request_id"], "0ABC");
    }
}
