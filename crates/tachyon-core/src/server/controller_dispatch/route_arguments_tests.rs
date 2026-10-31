use super::{RouteArguments, RouteRequest};
use crate::route_types::{ModelTable, Shape};
use crate::yon::plan::{ParameterSource, Route, RouteParameter};
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn parameter(source: ParameterSource, name: &str, shape: Shape) -> RouteParameter {
    RouteParameter {
        source,
        name: name.to_owned(),
        code_name: name.to_owned(),
        type_text: String::new(),
        shape,
    }
}

fn route(parameters: Vec<RouteParameter>) -> Route {
    Route {
        verb: String::from("POST"),
        method: String::from("placeOrder"),
        streaming: false,
        socket: None,
        takes_request: false,
        pattern: Some(String::from("/orders/:shop")),
        status: None,
        parameters,
        returns: String::new(),
        documentation: String::new(),
        return_shape: Shape::Any,
        error_responses: Vec::new(),
        dispatcher: false,
        access: crate::yon::route_access::RouteAccess::Public,
    }
}

fn order_models() -> ModelTable {
    let root = tempfile::tempdir().expect("project");
    let models = root.path().join("server/models");
    std::fs::create_dir_all(&models).expect("models folder");
    std::fs::write(
        models.join("Line.ts"),
        "interface Line { sku: string; count: number }\n",
    )
    .expect("Line model");
    std::fs::write(
        models.join("Order.ts"),
        "interface Order { id: string; lines: Line[]; note?: string }\n",
    )
    .expect("Order model");
    ModelTable::read(root.path(), "ts").expect("model table")
}

fn order_body() -> RouteParameter {
    parameter(
        ParameterSource::Body,
        "order",
        Shape::Model(String::from("Order")),
    )
}

fn build(route: &Route, headers: &HeaderMap, body: &[u8]) -> Result<Vec<Value>, Value> {
    let parameters = BTreeMap::from([(String::from("shop"), String::from("42"))]);
    let request = RouteRequest {
        parameters: &parameters,
        query: Some("tag=a&tag=b"),
        headers,
        body,
    };
    RouteArguments::build(route, &request, &order_models()).map_err(|rejection| {
        let answer = rejection.handler_response("request-1");
        assert_eq!(answer.status, 400);
        let body = answer.body.expect("rejection body");
        serde_json::from_str(&body.data).expect("JSON rejection")
    })
}

fn refused(route: &Route, headers: &HeaderMap, body: &[u8]) -> Value {
    match build(route, headers, body) {
        Ok(arguments) => panic!("the request was accepted: {arguments:?}"),
        Err(refusal) => refusal,
    }
}

fn assert_hidden(refusal: &Value, secrets: &[&str]) {
    let text = refusal.to_string();
    for secret in secrets {
        assert!(!text.contains(secret), "{secret} is in {text}");
    }
}

#[test]
fn a_request_that_fits_becomes_typed_arguments() {
    let route = route(vec![
        parameter(ParameterSource::Path, "shop", Shape::Integer),
        parameter(
            ParameterSource::Query,
            "tag",
            Shape::List(Box::new(Shape::Text)),
        ),
        parameter(ParameterSource::Header, "X-Count", Shape::Integer),
        parameter(ParameterSource::Cookie, "theme", Shape::Text),
        order_body(),
    ]);
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-count"),
        HeaderValue::from_static("3"),
    );
    headers.insert(COOKIE, HeaderValue::from_static("theme=dark; other=1"));
    let body = br#"{"id": "7", "lines": [{"sku": "a", "count": 2}]}"#;
    let arguments = build(&route, &headers, body).expect("arguments");
    assert_eq!(
        arguments,
        vec![
            json!(42),
            json!(["a", "b"]),
            json!(3),
            json!("dark"),
            json!({"id": "7", "lines": [{"sku": "a", "count": 2}]}),
        ]
    );
}

#[test]
fn a_body_with_an_undeclared_field_is_refused_without_naming_it() {
    let route = route(vec![order_body()]);
    for body in [
        br#"{"id": "7", "lines": [], "PRIVATE_SECRET": "hunter2"}"#.as_slice(),
        br#"{"id": "7", "lines": [{"sku": "a", "count": 1, "PRIVATE_SECRET": "hunter2"}]}"#,
    ] {
        let refusal = refused(&route, &HeaderMap::new(), body);
        assert_eq!(refusal["part"], json!("body"));
        assert!(
            refusal["error"]
                .as_str()
                .is_some_and(|error| error.contains("a field not declared in")),
            "{refusal}"
        );
        assert_hidden(&refusal, &["PRIVATE_SECRET", "hunter2"]);
    }
}

#[test]
fn a_body_value_that_does_not_fit_is_refused_without_echoing_it() {
    let route = route(vec![order_body()]);
    let body = br#"{"id": "7", "lines": [{"sku": "a", "count": "card 4111 1111 1111 1111"}]}"#;
    let refusal = refused(&route, &HeaderMap::new(), body);
    assert_eq!(
        refusal,
        json!({
            "error": "The request body does not fit: 'body.lines[0].count' must be a number.",
            "part": "body",
            "field": "",
        })
    );
    let required = refused(&route, &HeaderMap::new(), br#"{"lines": []}"#);
    assert_eq!(
        required["error"],
        json!("The request body does not fit: 'body.id' is required.")
    );
}

#[test]
fn a_body_that_is_not_json_or_is_missing_is_refused_without_echoing_it() {
    let route = route(vec![order_body()]);
    let broken = refused(&route, &HeaderMap::new(), b"{\"id\": \"hunter2\"");
    assert_eq!(
        broken["error"],
        json!("The request body is not valid JSON.")
    );
    assert_hidden(&broken, &["hunter2"]);
    let missing = refused(&route, &HeaderMap::new(), b"");
    assert_eq!(missing["error"], json!("The request body is required."));
}

#[test]
fn a_header_or_cookie_that_does_not_fit_is_refused_without_echoing_its_value() {
    let route = route(vec![
        parameter(ParameterSource::Header, "Authorization", Shape::Integer),
        parameter(ParameterSource::Cookie, "session", Shape::Integer),
    ]);
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Bearer secret-token-123"),
    );
    headers.insert(
        COOKIE,
        HeaderValue::from_static("session=cookie-secret-456"),
    );
    let header = refused(&route, &headers, b"");
    assert_eq!(
        header,
        json!({
            "error": "The header value 'Authorization' must be a whole number.",
            "part": "header",
            "field": "Authorization",
        })
    );
    headers.insert(AUTHORIZATION, HeaderValue::from_static("7"));
    let cookie = refused(&route, &headers, b"");
    assert_eq!(cookie["part"], json!("cookie"));
    assert_eq!(cookie["field"], json!("session"));
    assert_hidden(&cookie, &["cookie-secret-456"]);
}

#[test]
fn a_missing_required_value_names_its_part() {
    let route = route(vec![
        parameter(ParameterSource::Header, "X-Count", Shape::Integer),
        parameter(
            ParameterSource::Cookie,
            "theme",
            Shape::Optional(Box::new(Shape::Text)),
        ),
    ]);
    let refusal = refused(&route, &HeaderMap::new(), b"");
    assert_eq!(
        refusal,
        json!({
            "error": "The header value 'X-Count' is required.",
            "part": "header",
            "field": "X-Count",
        })
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-count"),
        HeaderValue::from_static("1"),
    );
    assert_eq!(
        build(&route, &headers, b""),
        Ok(vec![json!(1), Value::Null])
    );
}
