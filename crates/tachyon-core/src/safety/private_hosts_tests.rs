use super::PrivateHosts;
use serde_json::{Value, json};
use std::net::IpAddr;

fn address(text: &str) -> IpAddr {
    text.parse().unwrap_or(IpAddr::from([0, 0, 0, 0]))
}

fn listed() -> Vec<String> {
    vec![String::from("keycloak"), String::from("*.example.com")]
}

fn parsed(entries: &Value) -> Result<PrivateHosts, &'static str> {
    PrivateHosts::parse(Some(entries), &listed())
}

fn refused(entries: &Value) -> bool {
    parsed(entries).is_err()
}

fn keycloak(networks: &[&str]) -> Value {
    json!([{ "host": "keycloak", "networks": networks }])
}

#[test]
fn no_private_entries_admit_no_private_address() {
    let private = PrivateHosts::parse(None, &listed()).unwrap_or_default();

    assert!(!private.admits("keycloak", address("10.1.2.3")));
}

#[test]
fn an_entry_admits_addresses_inside_its_networks_only() {
    let private = parsed(&keycloak(&["172.16.0.0/12", "10.20.0.0/16"])).unwrap_or_default();

    assert!(private.admits("keycloak", address("172.18.0.4")));
    assert!(private.admits("keycloak", address("10.20.3.4")));
    assert!(private.admits("KEYCLOAK.", address("::ffff:172.18.0.4")));
    assert!(!private.admits("keycloak", address("10.21.0.1")));
    assert!(!private.admits("keycloak", address("192.168.1.1")));
    assert!(!private.admits("router", address("172.18.0.4")));
}

#[test]
fn a_single_address_is_a_network_of_one() {
    let private = parsed(&keycloak(&["192.168.8.20"])).unwrap_or_default();

    assert!(private.admits("keycloak", address("192.168.8.20")));
    assert!(!private.admits("keycloak", address("192.168.8.21")));
}

#[test]
fn metadata_addresses_stay_refused_inside_a_declared_network() {
    let private = parsed(&keycloak(&["100.64.0.0/10", "fc00::/7"])).unwrap_or_default();

    assert!(private.admits("keycloak", address("100.64.1.1")));
    assert!(private.admits("keycloak", address("fd12::1")));
    assert!(!private.admits("keycloak", address("100.100.100.200")));
    assert!(!private.admits("keycloak", address("fd00:ec2::254")));
}

#[test]
fn a_host_must_be_one_exact_listed_name() {
    assert!(refused(
        &json!([{ "host": "*.example.com", "networks": ["10.0.0.0/8"] }])
    ));
    assert!(refused(
        &json!([{ "host": "10.0.0.5", "networks": ["10.0.0.0/8"] }])
    ));
    assert!(refused(
        &json!([{ "host": "api.localhost", "networks": ["10.0.0.0/8"] }])
    ));
    assert!(refused(
        &json!([{ "host": "router", "networks": ["10.0.0.0/8"] }])
    ));
}

#[test]
fn networks_must_lie_inside_a_private_range() {
    assert!(refused(&keycloak(&[])));
    assert!(refused(&keycloak(&["0.0.0.0/0"])));
    assert!(refused(&keycloak(&["127.0.0.0/8"])));
    assert!(refused(&keycloak(&["169.254.0.0/16"])));
    assert!(refused(&keycloak(&["169.254.169.254"])));
    assert!(refused(&keycloak(&["fe80::/10"])));
    assert!(refused(&keycloak(&["::1"])));
    assert!(refused(&keycloak(&["8.8.8.0/24"])));
    assert!(refused(&keycloak(&["10.0.0.0/7"])));
    assert!(refused(&keycloak(&["::/0"])));
    assert!(refused(&keycloak(&["::ffff:10.0.0.0/104"])));
    assert!(refused(&keycloak(&["keycloak"])));
}

#[test]
fn an_entry_has_only_a_host_and_networks() {
    assert!(refused(
        &json!({ "host": "keycloak", "networks": ["10.0.0.0/8"] })
    ));
    assert!(refused(&json!(["keycloak"])));
    assert!(refused(&json!([{ "host": "keycloak" }])));
    assert!(refused(
        &json!([{ "host": "keycloak", "networks": "10.0.0.0/8" }])
    ));
    assert!(refused(&json!([{ "host": "keycloak", "networks": [8] }])));
    assert!(refused(
        &json!([{ "host": "keycloak", "networks": ["10.0.0.0/8"], "ports": [8080] }])
    ));
}

#[test]
fn a_host_is_named_once() {
    let twice = json!([
        { "host": "keycloak", "networks": ["10.0.0.0/8"] },
        { "host": "keycloak", "networks": ["172.16.0.0/12"] }
    ]);

    assert!(refused(&twice));
}
