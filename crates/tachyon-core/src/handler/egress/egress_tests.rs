use super::{EgressAddress, EgressProxy, EgressTarget};
use crate::safety::DeclaredHosts;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn through_proxy(proxy: &str, request: &str) -> String {
    let address = proxy.trim_start_matches("http://");
    let mut stream = TcpStream::connect(address).await.expect("proxy");
    stream.write_all(request.as_bytes()).await.expect("request");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).await.expect("answer");
    String::from_utf8_lossy(&answer).into_owned()
}

#[test]
fn proxy_requests_name_their_host() {
    let plain = EgressTarget::read("GET http://api.example.com:8080/items?page=2 HTTP/1.1\r\nHost: api.example.com\r\nProxy-Connection: keep-alive\r\n\r\n")
        .expect("plain target");
    assert_eq!(plain.host, "api.example.com");
    assert_eq!(plain.port, 8080);
    assert!(!plain.tunnel);
    assert_eq!(
        String::from_utf8(plain.head).expect("head"),
        "GET /items?page=2 HTTP/1.1\r\nHost: api.example.com\r\n\r\n"
    );
    let tunnel =
        EgressTarget::read("CONNECT API.example.com:443 HTTP/1.1\r\n\r\n").expect("tunnel");
    assert_eq!(
        (tunnel.host.as_str(), tunnel.port, tunnel.tunnel),
        ("api.example.com", 443, true)
    );
    let ipv6 = EgressTarget::read("CONNECT [::1]:443 HTTP/1.1\r\n\r\n").expect("ipv6");
    assert_eq!(ipv6.host, "::1");
    assert!(EgressTarget::read("GET /relative HTTP/1.1\r\n\r\n").is_none());
}

#[tokio::test]
async fn an_undeclared_host_is_refused_and_a_declared_host_is_reached() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.expect("upstream");
    let port = upstream.local_addr().expect("address").port();
    tokio::spawn(async move {
        let (mut stream, _) = upstream.accept().await.expect("accept");
        let mut request = vec![0_u8; 1024];
        let read = stream.read(&mut request).await.expect("read");
        let first = String::from_utf8_lossy(&request[..read])
            .lines()
            .next()
            .unwrap_or("")
            .to_owned();
        let body = format!("seen {first}");
        let answer = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(answer.as_bytes()).await.expect("answer");
    });
    let hosts = DeclaredHosts::listed(&["127.0.0.1"], "outgoing.hosts");
    let proxy = EgressProxy::start(hosts).await.expect("proxy");
    let refused = through_proxy(
        &proxy,
        "GET http://evil.example/secret HTTP/1.1\r\nHost: evil.example\r\n\r\n",
    )
    .await;
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");
    assert!(refused.contains("does not list this host"), "{refused}");
    let tunnel = through_proxy(&proxy, "CONNECT metadata.internal:443 HTTP/1.1\r\n\r\n").await;
    assert!(tunnel.starts_with("HTTP/1.1 403"), "{tunnel}");
    let reached = through_proxy(
        &proxy,
        &format!("GET http://127.0.0.1:{port}/items HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
    )
    .await;
    assert!(reached.contains("seen GET /items HTTP/1.1"), "{reached}");
}

async fn private_service() -> u16 {
    let upstream = TcpListener::bind("127.0.0.1:0").await.expect("upstream");
    let port = upstream.local_addr().expect("address").port();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = upstream.accept().await {
            let mut request = vec![0_u8; 1024];
            let _read = stream.read(&mut request).await;
            let answer = "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nreached";
            let _written = stream.write_all(answer.as_bytes()).await;
        }
    });
    port
}

#[tokio::test]
async fn a_listed_name_that_resolves_to_a_loopback_address_is_refused() {
    let port = private_service().await;
    let hosts = DeclaredHosts::listed(&["127.1"], "outgoing.hosts");
    let proxy = EgressProxy::start(hosts).await.expect("proxy");
    let plain = through_proxy(
        &proxy,
        &format!("GET http://127.1:{port}/ HTTP/1.1\r\nHost: 127.1\r\n\r\n"),
    )
    .await;
    assert!(plain.starts_with("HTTP/1.1 403"), "{plain}");
    assert!(!plain.contains("reached"), "{plain}");
    let tunnel = through_proxy(&proxy, &format!("CONNECT 127.1:{port} HTTP/1.1\r\n\r\n")).await;
    assert!(tunnel.starts_with("HTTP/1.1 403"), "{tunnel}");
}

#[test]
fn only_public_addresses_count_as_public() {
    for address in [
        "93.184.216.34",
        "8.8.8.8",
        "100.63.255.255",
        "100.128.0.1",
        "2606:4700::1111",
        "::ffff:93.184.216.34",
    ] {
        let address = address.parse().expect("address");
        assert!(EgressAddress::is_public(address), "{address}");
    }
    for address in [
        "127.0.0.1",
        "127.9.9.9",
        "10.0.0.5",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "100.100.100.200",
        "0.0.0.0",
        "0.1.2.3",
        "255.255.255.255",
        "::1",
        "::",
        "fd00::1",
        "fe80::1",
        "fec0::1",
        "::ffff:127.0.0.1",
        "::ffff:169.254.169.254",
    ] {
        let address = address.parse().expect("address");
        assert!(!EgressAddress::is_public(address), "{address}");
    }
}
