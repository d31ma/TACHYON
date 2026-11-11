use super::{
    EgressAddress, EgressConfinement, EgressEnforcement, EgressFraming, EgressProxy, EgressPump,
    EgressTarget, NodeProxyGate,
};
use crate::handler::{EnvironmentPolicy, HandlerRuntimePrograms};
use crate::safety::DeclaredHosts;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
        "GET /items?page=2 HTTP/1.1\r\nHost: api.example.com:8080\r\nConnection: close\r\n\r\n"
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
    let proxy = format!("http://{}", EgressProxy::start(hosts).await.expect("proxy"));
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
    let resolves = tokio::net::lookup_host(("127.1", port)).await.is_ok();
    let refusal = if resolves {
        "HTTP/1.1 403"
    } else {
        "HTTP/1.1 502"
    };
    let hosts = DeclaredHosts::listed(&["127.1"], "outgoing.hosts");
    let proxy = format!("http://{}", EgressProxy::start(hosts).await.expect("proxy"));
    let plain = through_proxy(
        &proxy,
        &format!("GET http://127.1:{port}/ HTTP/1.1\r\nHost: 127.1\r\n\r\n"),
    )
    .await;
    assert!(plain.starts_with(refusal), "{plain}");
    assert!(!plain.contains("reached"), "{plain}");
    let tunnel = through_proxy(&proxy, &format!("CONNECT 127.1:{port} HTTP/1.1\r\n\r\n")).await;
    assert!(tunnel.starts_with(refusal), "{tunnel}");
}

#[tokio::test]
async fn a_listed_localhost_name_reaches_loopback_without_asking_the_resolver() {
    let port = private_service().await;
    let hosts = DeclaredHosts::listed(&["*.localhost"], "outgoing.hosts");
    let proxy = format!("http://{}", EgressProxy::start(hosts).await.expect("proxy"));
    let plain = through_proxy(
        &proxy,
        &format!("GET http://api.localhost:{port}/ HTTP/1.1\r\nHost: api.localhost\r\n\r\n"),
    )
    .await;
    assert!(plain.ends_with("reached"), "{plain}");
    let tunnel = through_proxy(
        &proxy,
        &format!("CONNECT api.localhost:{port} HTTP/1.1\r\n\r\nGET / HTTP/1.1\r\n\r\n"),
    )
    .await;
    assert!(tunnel.ends_with("reached"), "{tunnel}");
    let candidates = EgressAddress::candidates("API.localhost.", port)
        .await
        .expect("candidates");
    let expected: Vec<std::net::SocketAddr> =
        [format!("[::1]:{port}"), format!("127.0.0.1:{port}")]
            .iter()
            .map(|address| address.parse().expect("address"))
            .collect();
    assert_eq!(candidates, expected);
}

#[tokio::test]
async fn an_unlisted_localhost_name_is_refused() {
    let port = private_service().await;
    let hosts = DeclaredHosts::listed(&["web.localhost"], "outgoing.hosts");
    let proxy = format!("http://{}", EgressProxy::start(hosts).await.expect("proxy"));
    let refused = through_proxy(
        &proxy,
        &format!("GET http://api.localhost:{port}/ HTTP/1.1\r\nHost: api.localhost\r\n\r\n"),
    )
    .await;
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");
    assert!(!refused.contains("reached"), "{refused}");
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
        "198.20.0.1",
        "192.0.1.1",
        "223.255.255.254",
        "2001:4860::8888",
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
        "198.18.0.1",
        "198.19.255.255",
        "192.0.0.170",
        "240.0.0.1",
        "224.0.0.251",
        "192.0.2.1",
        "64:ff9b::a9fe:a9fe",
        "64:ff9b:1::a00:1",
        "2002:a9fe:a9fe::1",
        "2001::1",
        "2001:db8::1",
        "::10.0.0.1",
        "::ffff:0:a9fe:a9fe",
        "100::1",
        "ff02::1",
    ] {
        let address = address.parse().expect("address");
        assert!(!EgressAddress::is_public(address), "{address}");
    }
}

#[cfg(unix)]
fn fake_javascript_runtime(directory: &std::path::Path, identity: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let program = directory.join("node");
    std::fs::write(&program, format!("#!/bin/sh\necho '{identity}'\n")).expect("fake runtime");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
        .expect("permissions");
    program
}

#[cfg(windows)]
fn fake_javascript_runtime(directory: &std::path::Path, identity: &str) -> std::path::PathBuf {
    let program = directory.join("node.cmd");
    std::fs::write(&program, format!("@echo off\r\necho {identity}\r\n")).expect("fake runtime");
    program
}

fn runtimes_with(javascript: std::path::PathBuf) -> HandlerRuntimePrograms {
    HandlerRuntimePrograms {
        javascript,
        ..HandlerRuntimePrograms::default()
    }
}

#[tokio::test]
async fn a_node_older_than_24_5_is_refused_before_it_runs_a_handler_outside_the_guard() {
    for identity in ["v23.5.0", "v24.4.1", "v22.21.0", "not a version"] {
        let directory = tempfile::tempdir().expect("fake runtime directory");
        let runtimes = runtimes_with(fake_javascript_runtime(directory.path(), identity));
        let refused = NodeProxyGate::default()
            .check(&runtimes, &EnvironmentPolicy::default())
            .await;
        let rendered = refused.expect_err(identity).to_string();
        assert!(rendered.contains("TY2101"), "{identity}: {rendered}");
        assert!(
            rendered.contains(
                "Required configured Yon JavaScript runtime (YON_JAVASCRIPT_RUNTIME) is not usable."
            ),
            "{identity}: {rendered}"
        );
        assert!(
            rendered.contains("Install Node.js 24.5 or newer, or Bun"),
            "{identity}: {rendered}"
        );
        assert!(
            !rendered.contains(directory.path().to_string_lossy().as_ref()),
            "{identity}: {rendered}"
        );
    }
}

#[tokio::test]
async fn node_24_5_or_newer_and_bun_pass_the_outgoing_guard_check() {
    for identity in ["v24.5.0", "v24.21.0", "v26.0.0", "bun"] {
        let directory = tempfile::tempdir().expect("fake runtime directory");
        let runtimes = runtimes_with(fake_javascript_runtime(directory.path(), identity));
        NodeProxyGate::default()
            .check(&runtimes, &EnvironmentPolicy::default())
            .await
            .expect(identity);
    }
}

#[tokio::test]
async fn a_missing_javascript_runtime_is_refused() {
    let directory = tempfile::tempdir().expect("empty directory");
    let runtimes = runtimes_with(directory.path().join("tachyon-missing-node"));
    let refused = NodeProxyGate::default()
        .check(&runtimes, &EnvironmentPolicy::default())
        .await;
    let rendered = refused.expect_err("missing runtime").to_string();
    assert!(rendered.contains("TY2112"), "{rendered}");
}

#[tokio::test]
async fn the_outgoing_guard_check_runs_the_runtime_once() {
    let directory = tempfile::tempdir().expect("fake runtime directory");
    let program = fake_javascript_runtime(directory.path(), "v24.5.0");
    let runtimes = runtimes_with(program.clone());
    let gate = NodeProxyGate::default();
    gate.check(&runtimes, &EnvironmentPolicy::default())
        .await
        .expect("first check");
    std::fs::remove_file(&program).expect("remove fake runtime");
    gate.clone()
        .check(&runtimes, &EnvironmentPolicy::default())
        .await
        .expect("remembered answer");
}

#[test]
fn the_enforcement_setting_accepts_only_os_or_proxy() {
    use std::ffi::OsStr;
    assert_eq!(
        EgressEnforcement::parse(None).expect("unset"),
        EgressEnforcement::Os
    );
    assert_eq!(
        EgressEnforcement::parse(Some(OsStr::new("os"))).expect("os"),
        EgressEnforcement::Os
    );
    assert_eq!(
        EgressEnforcement::parse(Some(OsStr::new("proxy"))).expect("proxy"),
        EgressEnforcement::Proxy
    );
    for value in ["", "off", "OS", "none"] {
        let refused = EgressEnforcement::parse(Some(OsStr::new(value))).expect_err(value);
        assert!(refused.to_string().contains("TY2113"), "{value}: {refused}");
    }
}

#[tokio::test]
async fn the_proxy_setting_leaves_handlers_unconfined() {
    let confinement = EgressConfinement::prepare(EgressEnforcement::Proxy, true, &[1, 2])
        .await
        .expect("proxy enforcement");
    assert!(!confinement.enforced());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn the_operating_system_confines_handlers_or_the_server_refuses_to_start() {
    let prepared = EgressConfinement::prepare(EgressEnforcement::Os, true, &[1, 2]).await;
    match prepared {
        Ok(confinement) => assert!(confinement.enforced()),
        Err(failure) => {
            assert_ne!(
                std::env::consts::OS,
                "macos",
                "macOS must always confine handlers: {failure}"
            );
            assert_ne!(
                std::env::var("YON_EGRESS_REQUIRE_CONFINEMENT").as_deref(),
                Ok("1"),
                "this host cannot confine handlers: {failure}"
            );
            assert!(failure.to_string().contains("TY2113"), "{failure}");
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[tokio::test]
async fn a_platform_without_a_confinement_layer_refuses_to_start_a_production_server() {
    let refused = EgressConfinement::prepare(EgressEnforcement::Os, true, &[1, 2])
        .await
        .expect_err("no layer");
    let rendered = refused.to_string();
    assert!(rendered.contains("TY2113"), "{rendered}");
    assert!(
        rendered.contains("YON_EGRESS_ENFORCEMENT=proxy"),
        "{rendered}"
    );
}

#[test]
fn a_production_server_refuses_to_run_handlers_it_cannot_confine() {
    let refused =
        EgressConfinement::settle(Err(String::from("no layer")), true).expect_err("refused");
    let rendered = refused.to_string();
    assert!(rendered.contains("TY2113"), "{rendered}");
    assert!(rendered.contains("no layer"), "{rendered}");
}

#[test]
fn a_development_server_warns_and_runs_handlers_it_cannot_confine() {
    let warned = EgressConfinement::settle(Err(String::from("no layer")), false).expect("warned");
    assert!(!warned.enforced());
    let event = EgressConfinement::unenforced_event("no layer");
    assert_eq!(event["event"], "handler.egress_unenforced");
    assert_eq!(event["level"], "warning");
    assert_eq!(event["reason"], "no layer");
}

#[cfg(target_os = "macos")]
#[test]
fn the_seatbelt_profile_allows_only_the_listed_loopback_ports() {
    let profile = super::SeatbeltProfile::text(&[4100, 4200]);
    assert_eq!(
        profile,
        "(version 1)(allow default)(deny network*)(deny appleevent-send)(deny mach-lookup (global-name \"com.apple.coreservices.launchservicesd\") (global-name \"com.apple.nsurlsessiond\") (global-name-prefix \"com.apple.lsd.\"))(allow network-outbound (remote ip \"localhost:4100\"))(allow network-outbound (remote ip \"localhost:4200\"))"
    );
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn a_confined_command_keeps_its_environment_and_directory() {
    let Ok(confinement) = EgressConfinement::prepare(EgressEnforcement::Os, true, &[1, 2]).await
    else {
        return;
    };
    let directory = tempfile::tempdir().expect("directory");
    let mut command = tokio::process::Command::new("sh");
    command
        .args(["-c", "printf '%s %s' \"$YON_PROBE\" \"$(pwd -P)\""])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("YON_PROBE", "kept")
        .current_dir(directory.path());
    confinement.apply(&mut command);
    assert_ne!(command.as_std().get_program(), "sh");
    let output = command.output().await.expect("confined command");
    let expected = format!(
        "kept {}",
        directory
            .path()
            .canonicalize()
            .expect("real path")
            .display()
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn a_program_that_is_not_on_the_path_is_still_confined() {
    let Ok(confinement) = EgressConfinement::prepare(EgressEnforcement::Os, true, &[1, 2]).await
    else {
        return;
    };
    let mut command = tokio::process::Command::new("tachyon-local-program");
    command.env_clear().env("PATH", "");
    confinement.apply(&mut command);
    let wrapped = command.as_std();
    assert_ne!(wrapped.get_program(), "tachyon-local-program");
    assert!(
        wrapped
            .get_args()
            .any(|argument| argument == "tachyon-local-program"),
        "{wrapped:?}"
    );
}

async fn connected_pair(listener: &TcpListener) -> (TcpStream, TcpStream) {
    let address = listener.local_addr().expect("address");
    let near = TcpStream::connect(address).await.expect("connect");
    let (far, _) = listener.accept().await.expect("accept");
    (near, far)
}

#[tokio::test]
async fn an_idle_tunnel_is_closed() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let (mut handler, mut guard_side) = connected_pair(&listener).await;
    let (mut upstream_side, mut service) = connected_pair(&listener).await;
    let pump = tokio::spawn(async move {
        EgressPump::exchange_until_idle(
            &mut guard_side,
            &mut upstream_side,
            std::time::Duration::from_millis(200),
        )
        .await
    });
    handler.write_all(b"ping").await.expect("ping");
    let mut forwarded = [0_u8; 4];
    service.read_exact(&mut forwarded).await.expect("forwarded");
    assert_eq!(&forwarded, b"ping");
    tokio::time::timeout(std::time::Duration::from_secs(5), pump)
        .await
        .expect("closed when idle")
        .expect("pump task")
        .expect("pump result");
}

async fn read_head_text(stream: &mut TcpStream) -> Option<String> {
    let mut bytes = Vec::new();
    let mut byte = [0_u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        let read = stream.read(&mut byte).await.ok()?;
        if read == 0 {
            return None;
        }
        bytes.push(byte[0]);
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

async fn read_answer(stream: &mut TcpStream) -> String {
    let head = read_head_text(stream).await.expect("answer head");
    let length = head
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .expect("content length");
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body).await.expect("body");
    format!("{head}{}", String::from_utf8_lossy(&body))
}

async fn keep_alive_service(name: &'static str, seen: Arc<Mutex<Vec<String>>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("service");
    let port = listener.local_addr().expect("address").port();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let seen = Arc::clone(&seen);
            tokio::spawn(async move {
                while let Some(head) = read_head_text(&mut stream).await {
                    seen.lock().expect("seen").push(head);
                    let answer = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: keep-alive\r\nKeep-Alive: timeout=60\r\n\r\n{name}",
                        name.len()
                    );
                    let _written = stream.write_all(answer.as_bytes()).await;
                }
            });
        }
    });
    port
}

#[tokio::test]
async fn a_kept_alive_proxy_connection_cannot_carry_a_request_to_a_second_host() {
    let first_seen = Arc::new(Mutex::new(Vec::new()));
    let second_seen = Arc::new(Mutex::new(Vec::new()));
    let first_port = keep_alive_service("first service", Arc::clone(&first_seen)).await;
    let second_port = keep_alive_service("second service", Arc::clone(&second_seen)).await;
    let hosts = DeclaredHosts::listed(&["127.0.0.1"], "outgoing.hosts");
    let guard = EgressProxy::start(hosts).await.expect("proxy");
    let second_request = format!(
        "GET http://127.0.0.1:{second_port}/jobs HTTP/1.1\r\nHost: 127.0.0.1:{second_port}\r\n\r\n"
    );
    let mut connection = TcpStream::connect(guard).await.expect("guard");
    connection
        .write_all(format!("POST http://127.0.0.1:{first_port}/token HTTP/1.1\r\nHost: 127.0.0.1:{first_port}\r\nConnection: keep-alive\r\nContent-Length: 0\r\n\r\n").as_bytes())
        .await
        .expect("first request");
    let first = read_answer(&mut connection).await;
    assert!(first.ends_with("first service"), "{first}");
    assert!(first.contains("\r\nConnection: close\r\n"), "{first}");
    assert!(
        !first.to_ascii_lowercase().contains("keep-alive"),
        "{first}"
    );
    let _sent = connection.write_all(second_request.as_bytes()).await;
    let mut second = Vec::new();
    let _read = tokio::time::timeout(Duration::from_secs(10), connection.read_to_end(&mut second))
        .await
        .expect("the guard closes the connection");
    let second = String::from_utf8_lossy(&second);
    assert!(!second.contains("service"), "{second}");
    assert_eq!(first_seen.lock().expect("first").len(), 1);
    assert!(second_seen.lock().expect("second").is_empty());
    let mut fresh = TcpStream::connect(guard).await.expect("guard");
    fresh
        .write_all(second_request.as_bytes())
        .await
        .expect("second request");
    let reached = read_answer(&mut fresh).await;
    assert!(reached.ends_with("second service"), "{reached}");
    let heads = second_seen.lock().expect("second").clone();
    assert_eq!(
        heads,
        vec![format!(
            "GET /jobs HTTP/1.1\r\nHost: 127.0.0.1:{second_port}\r\nConnection: close\r\n\r\n"
        )]
    );
}

async fn recording_service(seen: Arc<Mutex<Vec<u8>>>, end: &'static [u8]) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("service");
    let port = listener.local_addr().expect("address").port();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 1024];
        while !bytes.ends_with(end) {
            let read = stream.read(&mut chunk).await.expect("read");
            assert!(read > 0, "the request ended early");
            bytes.extend_from_slice(&chunk[..read]);
        }
        let more = tokio::time::timeout(Duration::from_millis(300), stream.read(&mut chunk)).await;
        let extra = more.ok().and_then(Result::ok).unwrap_or(0);
        bytes.extend_from_slice(&chunk[..extra]);
        seen.lock().expect("seen").extend_from_slice(&bytes);
        let answer = "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\nKeep-Alive: timeout=5\r\n\r\nok";
        let _written = stream.write_all(answer.as_bytes()).await;
    });
    port
}

#[tokio::test]
async fn a_chunked_request_reaches_its_own_host_and_the_answer_closes_the_connection() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let port = recording_service(Arc::clone(&seen), b"0\r\nTrailer: t\r\n\r\n").await;
    let hosts = DeclaredHosts::listed(&["127.0.0.1"], "outgoing.hosts");
    let proxy = format!("http://{}", EgressProxy::start(hosts).await.expect("proxy"));
    let answer = through_proxy(
        &proxy,
        &format!("POST http://127.0.0.1:{port}/upload HTTP/1.1\r\nHost: evil.example\r\nProxy-Authorization: Basic eA==\r\nConnection: keep-alive\r\nTransfer-Encoding: chunked\r\nExpect: 100-continue\r\n\r\n5;name=value\r\nhello\r\n0\r\nTrailer: t\r\n\r\n"),
    )
    .await;
    assert_eq!(
        answer,
        "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
    );
    let seen = String::from_utf8(seen.lock().expect("seen").clone()).expect("text");
    assert_eq!(
        seen,
        format!(
            "POST /upload HTTP/1.1\r\nTransfer-Encoding: chunked\r\nExpect: 100-continue\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n5;name=value\r\nhello\r\n0\r\nTrailer: t\r\n\r\n"
        )
    );
}

#[tokio::test]
async fn a_pipelined_second_request_never_reaches_the_upstream_service() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let port = recording_service(Arc::clone(&seen), b"hello").await;
    let hosts = DeclaredHosts::listed(&["127.0.0.1"], "outgoing.hosts");
    let proxy = format!("http://{}", EgressProxy::start(hosts).await.expect("proxy"));
    let mut connection = TcpStream::connect(proxy.trim_start_matches("http://"))
        .await
        .expect("guard");
    connection
        .write_all(format!("POST http://127.0.0.1:{port}/first HTTP/1.1\r\nContent-Length: 5\r\n\r\nhelloGET http://127.0.0.1:{port}/second HTTP/1.1\r\n\r\n").as_bytes())
        .await
        .expect("requests");
    let mut answer = Vec::new();
    let _read = tokio::time::timeout(Duration::from_secs(10), connection.read_to_end(&mut answer))
        .await
        .expect("the guard closes the connection");
    let answer = String::from_utf8_lossy(&answer);
    assert!(!answer.contains("200 OK"), "{answer}");
    tokio::time::sleep(Duration::from_millis(500)).await;
    let seen = String::from_utf8(seen.lock().expect("seen").clone()).expect("text");
    assert_eq!(
        seen,
        format!(
            "POST /first HTTP/1.1\r\nContent-Length: 5\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\nhello"
        )
    );
}

#[test]
fn a_plain_request_body_must_have_one_clear_length() {
    let framing = |fields: &str| {
        EgressTarget::read(&format!(
            "POST http://api.example/ HTTP/1.1\r\n{fields}\r\n"
        ))
        .map(|target| target.framing)
    };
    assert_eq!(framing(""), Some(EgressFraming::Length(0)));
    assert_eq!(
        framing("Content-Length: 5\r\n"),
        Some(EgressFraming::Length(5))
    );
    assert_eq!(
        framing("Content-Length: 5, 5\r\n"),
        Some(EgressFraming::Length(5))
    );
    assert_eq!(
        framing("Transfer-Encoding: gzip, Chunked\r\n"),
        Some(EgressFraming::Chunked)
    );
    assert_eq!(framing("Content-Length: 5\r\nContent-Length: 6\r\n"), None);
    assert_eq!(framing("Content-Length: +5\r\n"), None);
    assert_eq!(framing("Content-Length:\r\n"), None);
    assert_eq!(framing("Transfer-Encoding: chunked, gzip\r\n"), None);
    assert_eq!(
        framing("Transfer-Encoding: chunked\r\nContent-Length: 5\r\n"),
        None
    );
    assert_eq!(framing("Bad Name: value\r\n"), None);
    assert_eq!(framing("Accept: a\r\n folded\r\n"), None);
}

#[tokio::test]
async fn a_request_with_two_body_lengths_is_refused() {
    let hosts = DeclaredHosts::listed(&["127.0.0.1"], "outgoing.hosts");
    let proxy = format!("http://{}", EgressProxy::start(hosts).await.expect("proxy"));
    let refused = through_proxy(
        &proxy,
        "POST http://127.0.0.1:9/ HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\n",
    )
    .await;
    assert!(refused.starts_with("HTTP/1.1 400"), "{refused}");
}
