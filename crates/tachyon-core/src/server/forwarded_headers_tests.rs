use super::ForwardedHeaders;
use crate::safety::TrustedProxies;
use axum::http::{HeaderMap, HeaderValue};
use std::net::IpAddr;

fn spoofed() -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("x-forwarded-for", "6.6.6.6"),
        ("forwarded", "for=6.6.6.6;proto=https"),
        ("x-real-ip", "6.6.6.6"),
        ("x-forwarded-host", "bank.example"),
        ("x-forwarded-proto", "https"),
        ("x-forwarded-port", "443"),
        ("x-forwarded-prefix", "/admin"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    headers
}

fn edge(trusted: &[&str]) -> ForwardedHeaders {
    ForwardedHeaders::new(TrustedProxies::parse(trusted).expect("trusted proxies"))
}

fn address(text: &str) -> IpAddr {
    text.parse().expect("address")
}

fn value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

#[test]
fn a_direct_client_cannot_spoof_its_address_with_forwarding_headers() {
    let mut headers = spoofed();

    edge(&[]).apply(&mut headers, Some(address("203.0.113.9")));

    assert_eq!(value(&headers, "x-forwarded-for"), Some("203.0.113.9"));
    assert_eq!(value(&headers, "x-real-ip"), Some("203.0.113.9"));
    assert_eq!(
        value(&headers, "forwarded"),
        Some("for=203.0.113.9;proto=http")
    );
    assert_eq!(value(&headers, "x-forwarded-proto"), Some("http"));
    assert_eq!(value(&headers, "x-forwarded-host"), None);
    assert_eq!(value(&headers, "x-forwarded-port"), None);
    assert_eq!(value(&headers, "x-forwarded-prefix"), None);
}

#[test]
fn loopback_is_not_trusted_unless_it_is_listed() {
    let mut headers = spoofed();

    edge(&[]).apply(&mut headers, Some(address("127.0.0.1")));

    assert_eq!(value(&headers, "x-real-ip"), Some("127.0.0.1"));
    assert_eq!(value(&headers, "x-forwarded-for"), Some("127.0.0.1"));
}

#[test]
fn a_trusted_proxy_keeps_the_chain_and_names_the_nearest_untrusted_client() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("6.6.6.6, 198.51.100.4"),
    );
    headers.insert("forwarded", HeaderValue::from_static("for=198.51.100.4"));
    headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
    headers.insert("x-real-ip", HeaderValue::from_static("6.6.6.6"));

    edge(&["10.0.0.0/8"]).apply(&mut headers, Some(address("10.1.2.3")));

    assert_eq!(
        value(&headers, "x-forwarded-for"),
        Some("6.6.6.6, 198.51.100.4, 10.1.2.3")
    );
    assert_eq!(value(&headers, "x-real-ip"), Some("198.51.100.4"));
    assert_eq!(
        value(&headers, "forwarded"),
        Some("for=198.51.100.4, for=10.1.2.3;proto=http")
    );
    assert_eq!(value(&headers, "x-forwarded-proto"), Some("https"));
}

#[test]
fn a_chain_of_trusted_proxies_is_skipped_to_reach_the_client() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("198.51.100.4, 10.0.0.9"),
    );

    edge(&["10.0.0.0/8"]).apply(&mut headers, Some(address("10.1.2.3")));

    assert_eq!(value(&headers, "x-real-ip"), Some("198.51.100.4"));
}

#[test]
fn a_malformed_hop_stops_the_walk_at_the_last_trusted_proxy() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-forwarded-for",
        HeaderValue::from_static("198.51.100.4, not-an-address"),
    );

    edge(&["10.0.0.0/8"]).apply(&mut headers, Some(address("10.1.2.3")));

    assert_eq!(value(&headers, "x-real-ip"), Some("10.1.2.3"));
}

#[test]
fn an_ipv6_peer_is_quoted_in_the_forwarded_header() {
    let mut headers = HeaderMap::new();

    edge(&[]).apply(&mut headers, Some(address("2001:db8::7")));

    assert_eq!(
        value(&headers, "forwarded"),
        Some("for=\"[2001:db8::7]\";proto=http")
    );
    assert_eq!(value(&headers, "x-real-ip"), Some("2001:db8::7"));
}

#[test]
fn an_unknown_peer_removes_forwarding_headers_and_adds_none() {
    let mut headers = spoofed();

    edge(&[]).apply(&mut headers, None);

    assert!(headers.is_empty(), "{headers:?}");
}
