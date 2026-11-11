use super::EgressHead;
use std::io::{Error, ErrorKind};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

pub(super) struct EgressAnswer;

impl EgressAnswer {
    const MAX_HEAD: usize = 64 * 1024;
    const DROPPED: [&'static str; 2] = ["connection", "keep-alive"];
    const CLOSE: &'static str = "Connection: close";
    const FIRST_INFORMATION: u16 = 100;
    const SWITCHING_PROTOCOLS: u16 = 101;
    const FIRST_FINAL: u16 = 200;
    const INVALID: &'static str = "the upstream service sent an invalid HTTP answer head";

    pub(super) async fn relay_heads(
        upstream: &mut (impl AsyncRead + Unpin),
        client: &mut (impl AsyncWrite + Unpin),
    ) -> std::io::Result<Vec<u8>> {
        Self::relay_until_final(upstream, client).await
    }

    async fn relay_until_final(
        upstream: &mut (impl AsyncRead + Unpin),
        client: &mut (impl AsyncWrite + Unpin),
    ) -> std::io::Result<Vec<u8>> {
        let mut pending = Vec::new();
        let mut informational = true;
        while informational {
            let (head, rest) = Self::next_head(upstream, pending).await?;
            pending = rest;
            informational = Self::send_head(&head, client).await?;
        }
        Ok(pending)
    }

    async fn next_head(
        upstream: &mut (impl AsyncRead + Unpin),
        pending: Vec<u8>,
    ) -> std::io::Result<(String, Vec<u8>)> {
        EgressHead::read(upstream, pending, Self::MAX_HEAD)
            .await
            .ok_or_else(Self::invalid)
    }

    async fn send_head(
        head: &str,
        client: &mut (impl AsyncWrite + Unpin),
    ) -> std::io::Result<bool> {
        let informational = Self::informational(Self::status(head)?);
        let text = Self::outgoing(head, informational)?;
        client.write_all(text.as_bytes()).await?;
        Ok(informational)
    }

    fn status(head: &str) -> std::io::Result<u16> {
        head.split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .ok_or_else(Self::invalid)
    }

    fn informational(status: u16) -> bool {
        (Self::FIRST_INFORMATION..Self::FIRST_FINAL).contains(&status)
            && status != Self::SWITCHING_PROTOCOLS
    }

    fn outgoing(head: &str, informational: bool) -> std::io::Result<String> {
        if informational {
            return Ok(head.to_owned());
        }
        EgressHead::rewrite(head, &Self::DROPPED, &[String::from(Self::CLOSE)])
            .ok_or_else(Self::invalid)
    }

    fn invalid() -> Error {
        Error::new(ErrorKind::InvalidData, Self::INVALID)
    }
}
