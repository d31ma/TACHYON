use super::HostGuard;
use axum::http::header::HOST;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use std::net::{IpAddr, Ipv4Addr};

fn status(guard: &HostGuard, host: &str) -> StatusCode {
    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_str(host).expect("header"));
    guard
        .refusal_for(&headers)
        .map_or(StatusCode::OK, |refusal| refusal.status())
}

fn listed() -> Vec<String> {
    vec![
        String::from("app.example.com"),
        String::from("*.tunnel.example"),
    ]
}

#[test]
fn a_loopback_server_allows_local_names_addresses_the_origin_and_listed_names() {
    let origin = Some(String::from("shop.example"));
    let guard = HostGuard::new(IpAddr::V4(Ipv4Addr::LOCALHOST), &listed(), origin);
    for host in [
        "localhost:8080",
        "shop.localhost",
        "127.0.0.2",
        "[::1]:8080",
        "10.0.0.5",
        "shop.example",
        "shop.example.",
        "app.example.com",
        "APP.example.com:443",
        "x.tunnel.example",
    ] {
        assert_eq!(status(&guard, host), StatusCode::OK, "{host}");
    }
    for host in [
        "evil.example",
        "tunnel.example",
        "example.com",
        "shop.example.evil",
    ] {
        assert_eq!(status(&guard, host), StatusCode::FORBIDDEN, "{host}");
    }
    for host in ["bad host", "user@localhost", "[::1"] {
        assert_eq!(status(&guard, host), StatusCode::BAD_REQUEST, "{host}");
    }
}

#[test]
fn a_request_without_a_host_and_a_repeated_host_header() {
    let guard = HostGuard::new(IpAddr::V4(Ipv4Addr::LOCALHOST), &[], None);
    assert!(guard.refusal_for(&HeaderMap::new()).is_none());
    let mut repeated = HeaderMap::new();
    repeated.append(HOST, HeaderValue::from_static("localhost"));
    repeated.append(HOST, HeaderValue::from_static("evil.example"));
    let refusal = guard.refusal_for(&repeated).expect("repeated Host");
    assert_eq!(refusal.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn a_server_open_to_the_network_checks_hosts_only_when_server_hosts_is_set() {
    let open = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    let unchecked = HostGuard::new(open, &[], None);
    assert_eq!(status(&unchecked, "evil.example"), StatusCode::OK);
    let checked = HostGuard::new(open, &listed(), None);
    assert_eq!(status(&checked, "evil.example"), StatusCode::FORBIDDEN);
    assert_eq!(status(&checked, "app.example.com"), StatusCode::OK);
}
