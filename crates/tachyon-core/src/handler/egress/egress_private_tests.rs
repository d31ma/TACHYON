use super::{EgressAddress, EgressProxy, EgressResolver, EgressRules};
use crate::safety::{DeclaredHosts, PrivateHosts};
use serde_json::json;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn address(text: &str) -> IpAddr {
    text.parse().expect("address")
}

fn private_rules(host: &str, networks: &[&str], answers: &[IpAddr]) -> EgressRules {
    let hosts = vec![host.to_owned()];
    let entries = json!([{ "host": host, "networks": networks }]);
    let private = PrivateHosts::parse(Some(&entries), &hosts).expect("private hosts");
    let declared = DeclaredHosts::listed(&[host], "outgoing.hosts").with_private(private);
    EgressRules::new(declared, EgressResolver::fixed(answers))
}

fn listed_rules(host: &str, answers: &[IpAddr]) -> EgressRules {
    let declared = DeclaredHosts::listed(&[host], "outgoing.hosts");
    EgressRules::new(declared, EgressResolver::fixed(answers))
}

async fn kept(rules: &EgressRules, host: &str) -> Vec<IpAddr> {
    EgressAddress::candidates(host, 8080, rules)
        .await
        .expect("candidates")
        .iter()
        .map(SocketAddr::ip)
        .collect()
}

async fn through_guard(rules: EgressRules, request: &str) -> String {
    let guard = EgressProxy::start_with(rules).await.expect("guard");
    let mut stream = TcpStream::connect(guard).await.expect("guard connection");
    stream.write_all(request.as_bytes()).await.expect("request");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).await.expect("answer");
    String::from_utf8_lossy(&answer).into_owned()
}

fn own_private_address() -> IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("socket");
    socket
        .connect("10.255.255.254:9")
        .expect("this machine has a route to a private network");
    socket.local_addr().expect("local address").ip()
}

async fn answering_service(address: IpAddr) -> u16 {
    let listener = TcpListener::bind((address, 0)).await.expect("service");
    let port = listener.local_addr().expect("service address").port();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut request = vec![0_u8; 1024];
        let _read = stream.read(&mut request).await.expect("read");
        let answer = "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nreached";
        stream.write_all(answer.as_bytes()).await.expect("write");
    });
    port
}

#[tokio::test]
async fn an_opted_in_private_host_is_reached_through_the_guard() {
    let private = own_private_address();
    let port = answering_service(private).await;
    let network = private.to_string();
    let rules = private_rules("keycloak", &[network.as_str()], &[private]);

    let answer = through_guard(
        rules,
        &format!("GET http://keycloak:{port}/idp HTTP/1.1\r\nHost: keycloak:{port}\r\n\r\n"),
    )
    .await;

    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.ends_with("reached"), "{answer}");
}

#[tokio::test]
async fn a_listed_host_without_a_private_entry_is_refused_on_a_private_address() {
    let rules = listed_rules("router", &[address("10.0.0.5")]);

    let answer = through_guard(
        rules,
        "GET http://router:8080/ HTTP/1.1\r\nHost: router:8080\r\n\r\n",
    )
    .await;

    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert!(answer.contains("yon.outgoing.private"), "{answer}");
}

#[tokio::test]
async fn an_opted_in_host_that_rebinds_to_the_metadata_address_is_refused_through_the_guard() {
    let rules = private_rules("keycloak", &["10.0.0.0/8"], &[address("169.254.169.254")]);

    let answer = through_guard(rules, "CONNECT keycloak:443 HTTP/1.1\r\n\r\n").await;

    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
}

#[tokio::test]
async fn an_opted_in_host_keeps_only_addresses_inside_its_networks() {
    let rules = private_rules(
        "keycloak",
        &["10.0.0.0/8"],
        &[
            address("10.1.2.3"),
            address("::ffff:10.1.2.4"),
            address("172.17.0.9"),
        ],
    );

    let kept = kept(&rules, "keycloak").await;

    assert_eq!(kept, vec![address("10.1.2.3"), address("::ffff:10.1.2.4")]);
}

#[tokio::test]
async fn an_opted_in_host_that_rebinds_outside_its_network_is_refused() {
    let rules = private_rules("keycloak", &["10.0.0.0/8"], &[address("172.17.0.9")]);

    assert!(kept(&rules, "keycloak").await.is_empty());
}

#[tokio::test]
async fn an_opted_in_host_that_rebinds_to_loopback_is_refused() {
    let rules = private_rules(
        "keycloak",
        &["10.0.0.0/8"],
        &[address("127.0.0.1"), address("::1")],
    );

    assert!(kept(&rules, "keycloak").await.is_empty());
}

#[tokio::test]
async fn an_opted_in_host_that_rebinds_to_link_local_addresses_is_refused() {
    let rules = private_rules(
        "keycloak",
        &["10.0.0.0/8"],
        &[
            address("169.254.169.254"),
            address("169.254.170.2"),
            address("fe80::1"),
        ],
    );

    assert!(kept(&rules, "keycloak").await.is_empty());
}

#[tokio::test]
async fn a_metadata_address_inside_a_shared_network_is_refused() {
    let rules = private_rules(
        "keycloak",
        &["100.64.0.0/10"],
        &[address("100.100.100.200")],
    );

    assert!(kept(&rules, "keycloak").await.is_empty());
}

#[tokio::test]
async fn a_metadata_address_inside_a_unique_local_network_is_refused() {
    let rules = private_rules("keycloak", &["fc00::/7"], &[address("fd00:ec2::254")]);

    assert!(kept(&rules, "keycloak").await.is_empty());
}

#[tokio::test]
async fn a_private_entry_does_not_cover_another_listed_name() {
    let rules = private_rules("keycloak", &["10.0.0.0/8"], &[address("10.1.2.3")]);

    assert!(kept(&rules, "keycloak.example.com").await.is_empty());
}

#[tokio::test]
async fn a_public_address_stays_allowed_beside_a_refused_private_one() {
    let rules = private_rules(
        "keycloak",
        &["10.0.0.0/8"],
        &[address("93.184.216.34"), address("192.168.1.10")],
    );

    assert_eq!(
        kept(&rules, "keycloak").await,
        vec![address("93.184.216.34")]
    );
}

#[tokio::test]
async fn a_listed_host_without_a_private_entry_keeps_no_private_address() {
    let rules = listed_rules("router", &[address("10.0.0.5"), address("fd12::5")]);

    assert!(kept(&rules, "router").await.is_empty());
}
