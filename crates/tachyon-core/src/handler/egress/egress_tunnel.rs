use super::{EgressAddress, EgressHead, EgressPump, EgressRules, EgressTarget};
use crate::safety::DeclaredHosts;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub(super) struct EgressTunnel;

impl EgressTunnel {
    const MAX_HEAD: usize = 16 * 1024;
    const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
    const ESTABLISHED: &'static [u8] = b"HTTP/1.1 200 Connection Established\r\n\r\n";
    const BAD_REQUEST: &'static str =
        "HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
    const BUSY: &'static [u8] =
        b"HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
    const UNREACHABLE: &'static str =
        "HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

    pub(super) async fn serve(mut client: TcpStream, rules: Arc<EgressRules>) {
        let head = tokio::time::timeout(
            Self::HEAD_TIMEOUT,
            EgressHead::read(&mut client, Vec::new(), Self::MAX_HEAD),
        )
        .await;
        let target = Self::target(head.ok().flatten());
        let _ = Self::relay(client, target, &rules).await;
    }

    pub(super) async fn refuse_busy(mut client: TcpStream) {
        let _written = client.write_all(Self::BUSY).await;
    }

    fn target(head: Option<(String, Vec<u8>)>) -> Option<(EgressTarget, Vec<u8>)> {
        let (text, rest) = head?;
        Some((EgressTarget::read(&text)?, rest))
    }

    async fn relay(
        mut client: TcpStream,
        target: Option<(EgressTarget, Vec<u8>)>,
        rules: &EgressRules,
    ) -> std::io::Result<()> {
        match Self::decision(target, rules.hosts()) {
            Ok((target, rest)) => Self::connect(client, &target, &rest, rules).await,
            Err(answer) => client.write_all(answer.as_bytes()).await,
        }
    }

    fn decision(
        target: Option<(EgressTarget, Vec<u8>)>,
        hosts: &DeclaredHosts,
    ) -> Result<(EgressTarget, Vec<u8>), String> {
        let (target, rest) = target.ok_or_else(|| String::from(Self::BAD_REQUEST))?;
        Self::allowed(target, rest, hosts)
    }

    fn allowed(
        target: EgressTarget,
        rest: Vec<u8>,
        hosts: &DeclaredHosts,
    ) -> Result<(EgressTarget, Vec<u8>), String> {
        if hosts.allows(&target.host) {
            return Ok((target, rest));
        }
        Err(Self::refusal(&target, hosts))
    }

    async fn connect(
        client: TcpStream,
        target: &EgressTarget,
        rest: &[u8],
        rules: &EgressRules,
    ) -> std::io::Result<()> {
        let upstream = Self::open(target, rules).await;
        Self::bridge(client, upstream, target, rest).await
    }

    async fn open(target: &EgressTarget, rules: &EgressRules) -> Result<TcpStream, String> {
        let addresses = EgressAddress::candidates(&target.host, target.port, rules)
            .await
            .map_err(|_| String::from(Self::UNREACHABLE))?;
        let addresses = Some(addresses)
            .filter(|addresses| !addresses.is_empty())
            .ok_or_else(|| Self::private_refusal(target, rules.hosts()))?;
        let stream = TcpStream::connect(addresses.as_slice())
            .await
            .map_err(|_| String::from(Self::UNREACHABLE))?;
        rules.note_connection(&target.host, &stream);
        Ok(stream)
    }

    async fn bridge(
        mut client: TcpStream,
        upstream: Result<TcpStream, String>,
        target: &EgressTarget,
        rest: &[u8],
    ) -> std::io::Result<()> {
        match upstream {
            Ok(mut upstream) => Self::pipe(&mut client, &mut upstream, target, rest).await,
            Err(answer) => client.write_all(answer.as_bytes()).await,
        }
    }

    async fn pipe(
        client: &mut TcpStream,
        upstream: &mut TcpStream,
        target: &EgressTarget,
        rest: &[u8],
    ) -> std::io::Result<()> {
        client.write_all(Self::client_greeting(target)).await?;
        upstream.write_all(Self::upstream_head(target)).await?;
        Self::exchange(client, upstream, target, rest).await
    }

    async fn exchange(
        client: &mut TcpStream,
        upstream: &mut TcpStream,
        target: &EgressTarget,
        rest: &[u8],
    ) -> std::io::Result<()> {
        if target.tunnel {
            upstream.write_all(rest).await?;
            return EgressPump::exchange(client, upstream).await;
        }
        EgressPump::exchange_once(client, upstream, rest.to_vec(), target.framing).await
    }

    fn client_greeting(target: &EgressTarget) -> &'static [u8] {
        if target.tunnel {
            return Self::ESTABLISHED;
        }
        &[]
    }

    fn upstream_head(target: &EgressTarget) -> &[u8] {
        if target.tunnel {
            return &[];
        }
        &target.head
    }

    fn refusal(target: &EgressTarget, hosts: &DeclaredHosts) -> String {
        let message = format!(
            "Tachyon refused a request to {}: {} does not list this host (ADR 0068).",
            target.host,
            hosts.setting()
        );
        Self::forbidden(&message)
    }

    fn private_refusal(target: &EgressTarget, hosts: &DeclaredHosts) -> String {
        let message = format!(
            "Tachyon refused a request to {}: the name resolves only to loopback, private or link-local addresses that are not allowed for it. To reach a service on a private network by name, add the name and its network to yon.outgoing.private; to reach a loopback service, list a localhost name or its IP address in {} (ADR 0094).",
            target.host,
            hosts.setting()
        );
        Self::forbidden(&message)
    }

    fn forbidden(message: &str) -> String {
        format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{message}",
            message.len()
        )
    }
}
