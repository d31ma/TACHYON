use super::{EgressActivity, EgressFraming};
use std::io::{Cursor, Error, ErrorKind};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};

pub(super) struct EgressRequest;

impl EgressRequest {
    const CHUNK: usize = 16 * 1024;
    const MAX_LINE: u64 = 8 * 1024;
    const LINE_END: &'static [u8] = b"\r\n";
    const NEW_LINE: u8 = b'\n';
    const EXTENSION: char = ';';
    const HEX: u32 = 16;
    const INVALID_CHUNK: &'static str = "the request body has an invalid chunk";
    const SECOND_REQUEST: &'static str =
        "the outgoing request guard carries one request on each plain HTTP connection";

    pub(super) async fn send(
        client: impl AsyncRead + Unpin,
        mut upstream: impl AsyncWrite + Unpin,
        early: Vec<u8>,
        framing: EgressFraming,
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        let mut client = BufReader::new(Cursor::new(early).chain(client));
        Self::body(&mut client, &mut upstream, framing, activity).await?;
        upstream.flush().await?;
        Self::refuse_more(&mut client).await?;
        upstream.shutdown().await
    }

    async fn body(
        client: &mut (impl AsyncBufRead + Unpin),
        upstream: &mut (impl AsyncWrite + Unpin),
        framing: EgressFraming,
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        match framing {
            EgressFraming::Length(length) => {
                Self::copy_exact(client, upstream, length, activity).await
            }
            EgressFraming::Chunked => Self::copy_chunks(client, upstream, activity).await,
        }
    }

    async fn copy_exact(
        client: &mut (impl AsyncBufRead + Unpin),
        upstream: &mut (impl AsyncWrite + Unpin),
        length: u64,
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        let mut limited = (&mut *client).take(length);
        let mut chunk = vec![0_u8; Self::CHUNK];
        while Self::copy_step(&mut limited, upstream, &mut chunk, activity).await? {}
        Self::complete(limited.limit())
    }

    async fn copy_step(
        reader: &mut (impl AsyncRead + Unpin),
        writer: &mut (impl AsyncWrite + Unpin),
        chunk: &mut [u8],
        activity: &EgressActivity,
    ) -> std::io::Result<bool> {
        let read = reader.read(chunk).await?;
        writer.write_all(&chunk[..read]).await?;
        activity.mark();
        Ok(read > 0)
    }

    fn complete(left: u64) -> std::io::Result<()> {
        if left == 0 {
            return Ok(());
        }
        Err(Error::from(ErrorKind::UnexpectedEof))
    }

    async fn copy_chunks(
        client: &mut (impl AsyncBufRead + Unpin),
        upstream: &mut (impl AsyncWrite + Unpin),
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        let mut size = Self::copy_size(client, upstream, activity).await?;
        while size > 0 {
            Self::copy_exact(client, upstream, size, activity).await?;
            Self::copy_empty_line(client, upstream, activity).await?;
            size = Self::copy_size(client, upstream, activity).await?;
        }
        Self::copy_trailers(client, upstream, activity).await
    }

    async fn copy_size(
        client: &mut (impl AsyncBufRead + Unpin),
        upstream: &mut (impl AsyncWrite + Unpin),
        activity: &EgressActivity,
    ) -> std::io::Result<u64> {
        let line = Self::copy_line(client, upstream, activity).await?;
        Self::size(&line).ok_or_else(Self::invalid_chunk)
    }

    async fn copy_empty_line(
        client: &mut (impl AsyncBufRead + Unpin),
        upstream: &mut (impl AsyncWrite + Unpin),
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        let line = Self::copy_line(client, upstream, activity).await?;
        Self::empty(&line)
    }

    async fn copy_trailers(
        client: &mut (impl AsyncBufRead + Unpin),
        upstream: &mut (impl AsyncWrite + Unpin),
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        let mut line = Self::copy_line(client, upstream, activity).await?;
        while line != Self::LINE_END {
            line = Self::copy_line(client, upstream, activity).await?;
        }
        Ok(())
    }

    async fn copy_line(
        client: &mut (impl AsyncBufRead + Unpin),
        upstream: &mut (impl AsyncWrite + Unpin),
        activity: &EgressActivity,
    ) -> std::io::Result<Vec<u8>> {
        let mut line = Vec::new();
        (&mut *client)
            .take(Self::MAX_LINE)
            .read_until(Self::NEW_LINE, &mut line)
            .await?;
        let line = Self::whole_line(line)?;
        upstream.write_all(&line).await?;
        activity.mark();
        Ok(line)
    }

    fn whole_line(line: Vec<u8>) -> std::io::Result<Vec<u8>> {
        if line.ends_with(Self::LINE_END) {
            return Ok(line);
        }
        Err(Self::invalid_chunk())
    }

    fn empty(line: &[u8]) -> std::io::Result<()> {
        if line == Self::LINE_END {
            return Ok(());
        }
        Err(Self::invalid_chunk())
    }

    fn size(line: &[u8]) -> Option<u64> {
        let text = std::str::from_utf8(line).ok()?;
        let digits = text.trim_end().split(Self::EXTENSION).next()?.trim();
        Self::hex(digits).then(|| u64::from_str_radix(digits, Self::HEX).ok())?
    }

    fn hex(digits: &str) -> bool {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    }

    async fn refuse_more(client: &mut (impl AsyncRead + Unpin)) -> std::io::Result<()> {
        let mut probe = [0_u8; 1];
        let read = client.read(&mut probe).await?;
        Self::nothing_more(read)
    }

    fn nothing_more(read: usize) -> std::io::Result<()> {
        if read == 0 {
            return Ok(());
        }
        Err(Error::new(ErrorKind::InvalidData, Self::SECOND_REQUEST))
    }

    fn invalid_chunk() -> Error {
        Error::new(ErrorKind::InvalidData, Self::INVALID_CHUNK)
    }
}
