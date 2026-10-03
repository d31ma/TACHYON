use super::ServerHost;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[test]
fn a_name_under_localhost_binds_the_ipv4_loopback_address_and_keeps_its_name() {
    let host = ServerHost::parse("Shop.LocalHost.");
    let bound = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    assert_eq!(
        host.as_ref().map(ServerHost::address),
        Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
    );
    assert_eq!(
        host.as_ref().map(ServerHost::setting).as_deref(),
        Some("shop.localhost")
    );
    assert_eq!(
        host.map(|host| host.url(bound)).as_deref(),
        Some("http://shop.localhost:8080/")
    );
}

#[test]
fn localhost_and_addresses_print_the_bound_address() {
    let bound = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 3000);
    for text in ["localhost", "LOCALHOST", "127.0.0.1", "::1"] {
        let url = ServerHost::parse(text).map(|host| host.url(bound));
        assert_eq!(url.as_deref(), Some("http://[::1]:3000/"), "{text}");
    }
    let any = ServerHost::parse("0.0.0.0").map(|host| host.setting());
    assert_eq!(any.as_deref(), Some("0.0.0.0"));
}

#[test]
fn other_names_are_not_server_hosts() {
    for text in [
        "example.com",
        "localhost.example",
        ".localhost",
        "a..localhost",
        "",
    ] {
        assert_eq!(ServerHost::parse(text), None, "{text}");
    }
}
