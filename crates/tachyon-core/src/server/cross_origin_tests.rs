use super::CrossOrigin;
use crate::safety::AppOrigins;
use axum::Router;
use axum::body::Body;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, ORIGIN, VARY,
};
use axum::http::{Method, Request, Response, StatusCode};
use tower::ServiceExt as _;

fn application(origins: &[&str]) -> Router {
    let origins = AppOrigins::parse(origins).expect("origins");
    let routes = Router::new().fallback(axum::routing::any(|| async { "route answer" }));
    CrossOrigin::new(&origins).protect(routes)
}

async fn send(application: Router, request: Request<Body>) -> Response<Body> {
    application.oneshot(request).await.expect("response")
}

fn request(method: Method, origin: Option<&str>) -> axum::http::request::Builder {
    let builder = Request::builder().method(method).uri("/api/notes");
    match origin {
        Some(origin) => builder.header(ORIGIN, origin),
        None => builder,
    }
}

fn header<'a>(response: &'a Response<Body>, name: &axum::http::HeaderName) -> Option<&'a str> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
}

#[tokio::test]
async fn a_listed_app_origin_may_read_the_answer() {
    let app = application(&["tachyon-app://bundle", "https://tachyon.local"]);
    let response = send(
        app,
        request(Method::GET, Some("tachyon-app://bundle"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header(&response, &ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("tachyon-app://bundle")
    );
    assert_eq!(header(&response, &VARY), Some("Origin"));
}

#[tokio::test]
async fn a_preflight_from_a_listed_origin_is_answered_before_the_route() {
    let app = application(&["https://appassets.tachyon.local"]);
    let response = send(
        app,
        request(Method::OPTIONS, Some("https://appassets.tachyon.local"))
            .header(ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                ACCESS_CONTROL_REQUEST_HEADERS,
                "authorization, content-type",
            )
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        header(&response, &ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("https://appassets.tachyon.local")
    );
    assert_eq!(
        header(&response, &ACCESS_CONTROL_ALLOW_METHODS),
        Some("GET, HEAD, POST, PUT, PATCH, DELETE")
    );
    assert_eq!(
        header(&response, &ACCESS_CONTROL_ALLOW_HEADERS),
        Some("authorization, content-type")
    );
}

#[tokio::test]
async fn a_preflight_that_asks_for_malformed_headers_gets_no_header_grant() {
    let app = application(&["tachyon://app"]);
    let response = send(
        app,
        request(Method::OPTIONS, Some("tachyon://app"))
            .header(ACCESS_CONTROL_REQUEST_METHOD, "PUT")
            .header(ACCESS_CONTROL_REQUEST_HEADERS, "authorization, bad header")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(header(&response, &ACCESS_CONTROL_ALLOW_HEADERS), None);
}

#[tokio::test]
async fn an_unlisted_origin_reaches_the_route_without_a_grant() {
    for origin in [Some("https://evil.example"), Some("null"), None] {
        let app = application(&["https://tachyon.local"]);
        let preflight = send(
            app.clone(),
            request(Method::OPTIONS, origin)
                .header(ACCESS_CONTROL_REQUEST_METHOD, "DELETE")
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(preflight.status(), StatusCode::OK, "{origin:?}");
        assert_eq!(header(&preflight, &ACCESS_CONTROL_ALLOW_ORIGIN), None);
        assert_eq!(header(&preflight, &ACCESS_CONTROL_ALLOW_METHODS), None);
        let read = send(
            app,
            request(Method::GET, origin)
                .body(Body::empty())
                .expect("request"),
        )
        .await;
        assert_eq!(header(&read, &ACCESS_CONTROL_ALLOW_ORIGIN), None);
        assert_eq!(header(&read, &VARY), Some("Origin"));
    }
}

#[tokio::test]
async fn without_listed_origins_the_server_adds_no_cross_origin_headers() {
    let app = application(&[]);
    let response = send(
        app,
        request(Method::GET, Some("tachyon-app://bundle"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(header(&response, &ACCESS_CONTROL_ALLOW_ORIGIN), None);
    assert_eq!(header(&response, &VARY), None);
}

#[tokio::test]
async fn the_native_preview_app_may_read_the_development_server() {
    let origins = AppOrigins::parse(&[] as &[&str])
        .expect("origins")
        .with_origin("tachyon://app");
    let routes = Router::new().fallback(axum::routing::any(|| async { "route answer" }));
    let response = send(
        CrossOrigin::new(&origins).protect(routes),
        request(Method::GET, Some("tachyon://app"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header(&response, &ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("tachyon://app")
    );
}
