use super::TrustedProxies;
use std::net::IpAddr;

fn address(text: &str) -> IpAddr {
    text.parse().unwrap_or(IpAddr::from([0, 0, 0, 0]))
}

#[test]
fn single_addresses_and_ranges_match_their_members() {
    let trusted =
        TrustedProxies::parse(&["10.0.0.0/8", "192.0.2.7", "2001:db8::/32"]).unwrap_or_default();

    assert!(trusted.contains(address("10.20.30.40")));
    assert!(trusted.contains(address("192.0.2.7")));
    assert!(trusted.contains(address("2001:db8::1")));
    assert!(trusted.contains(address("::ffff:10.1.2.3")));
    assert!(!trusted.contains(address("192.0.2.8")));
    assert!(!trusted.contains(address("11.0.0.1")));
    assert!(!trusted.contains(address("2001:db9::1")));
}

#[test]
fn an_empty_list_trusts_no_proxy() {
    let trusted = TrustedProxies::parse(&[]).unwrap_or_default();

    assert!(!trusted.contains(address("127.0.0.1")));
}

#[test]
fn a_zero_prefix_matches_every_address_of_its_family() {
    let trusted = TrustedProxies::parse(&["0.0.0.0/0"]).unwrap_or_default();

    assert!(trusted.contains(address("203.0.113.9")));
    assert!(!trusted.contains(address("2001:db8::1")));
}

#[test]
fn malformed_entries_are_refused() {
    for entry in ["proxy.example", "10.0.0.0/33", "::1/129", "10.0.0.1/x", ""] {
        assert!(TrustedProxies::parse(&[entry]).is_none(), "{entry}");
    }
}
