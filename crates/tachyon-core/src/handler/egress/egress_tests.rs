use super::{EgressProxy, EgressTarget};
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
