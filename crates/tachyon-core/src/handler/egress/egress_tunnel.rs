use super::{EgressAddress, EgressTarget};
use crate::safety::DeclaredHosts;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(super) struct EgressTunnel;

impl EgressTunnel {
    const HEAD_END: &'static [u8] = b"\r\n\r\n";
    const MAX_HEAD: usize = 16 * 1024;
    const READ_CHUNK: usize = 4096;
    const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
    const ESTABLISHED: &'static [u8] = b"HTTP/1.1 200 Connection Established\r\n\r\n";
    const BAD_REQUEST: &'static str =
        "HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
    const UNREACHABLE: &'static str =
        "HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

    pub(super) async fn serve(mut client: TcpStream, hosts: Arc<DeclaredHosts>) {
        let head = tokio::time::timeout(Self::HEAD_TIMEOUT, Self::read_head(&mut client)).await;
        let target = Self::target(head.ok().flatten());
        let _ = Self::relay(client, target, &hosts).await;
    }

    async fn read_head(client: &mut TcpStream) -> Option<(String, Vec<u8>)> {
        let mut bytes = Vec::new();
        let mut chunk = vec![0_u8; Self::READ_CHUNK];
        while Self::head_end(&bytes).is_none() {
            let read = client
                .read(&mut chunk)
                .await
                .ok()
                .filter(|read| *read > 0)?;
            bytes.extend_from_slice(&chunk[..read]);
            (bytes.len() <= Self::MAX_HEAD).then_some(())?;
        }
        Self::split_head(bytes)
    }

    fn head_end(bytes: &[u8]) -> Option<usize> {
        bytes
            .windows(Self::HEAD_END.len())
            .position(|window| window == Self::HEAD_END)
    }

    fn split_head(mut bytes: Vec<u8>) -> Option<(String, Vec<u8>)> {
        let end = Self::head_end(&bytes)? + Self::HEAD_END.len();
        let rest = bytes.split_off(end);
        let head = String::from_utf8(bytes).ok()?;
        Some((head, rest))
    }

    fn target(head: Option<(String, Vec<u8>)>) -> Option<(EgressTarget, Vec<u8>)> {
        let (text, rest) = head?;
        Some((EgressTarget::read(&text)?, rest))
    }

    async fn relay(
        mut client: TcpStream,
        target: Option<(EgressTarget, Vec<u8>)>,
        hosts: &DeclaredHosts,
    ) -> std::io::Result<()> {
        match Self::decision(target, hosts) {
            Ok((target, rest)) => Self::connect(client, &target, &rest, hosts).await,
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
        hosts: &DeclaredHosts,
    ) -> std::io::Result<()> {
        let upstream = Self::open(target, hosts).await;
        Self::bridge(client, upstream, target, rest).await
    }

    async fn open(target: &EgressTarget, hosts: &DeclaredHosts) -> Result<TcpStream, String> {
        let addresses = EgressAddress::candidates(&target.host, target.port)
            .await
            .map_err(|_| String::from(Self::UNREACHABLE))?;
        let addresses = Some(addresses)
            .filter(|addresses| !addresses.is_empty())
            .ok_or_else(|| Self::private_refusal(target, hosts))?;
        TcpStream::connect(addresses.as_slice())
            .await
            .map_err(|_| String::from(Self::UNREACHABLE))
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
        upstream.write_all(rest).await?;
        tokio::io::copy_bidirectional(client, upstream).await?;
        Ok(())
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
            "Tachyon refused a request to {}: the name resolves only to loopback, private or link-local addresses. To reach a local or private service, list its IP address or a localhost name in {} (ADR 0094).",
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
