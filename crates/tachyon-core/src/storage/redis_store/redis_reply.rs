use std::io;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt};

use super::ManagedStorage;

/// A RESP2 reply to one of the commands Tachyon sends: a status, an error, an integer or a
/// (possibly nil) bulk string. Anything else (an array, a RESP3 type, a line or bulk string over
/// the limits) is a protocol error that closes the connection.
#[derive(Default)]
pub(super) struct RedisReply {
    error: bool,
    integer: Option<i64>,
    bytes: Option<Vec<u8>>,
}

impl RedisReply {
    const MAX_LINE: u64 = 64 * 1024;
    /// `GETRANGE key 0 MAX_BYTES` returns at most `MAX_BYTES + 1` bytes.
    const MAX_BULK: usize = ManagedStorage::MAX_BYTES + 1;

    pub(super) async fn read<R>(reader: &mut R) -> io::Result<Self>
    where
        R: AsyncBufRead + Unpin,
    {
        let line = Self::line(reader).await?;
        let (kind, rest) = line.split_first().ok_or_else(Self::out_of_step)?;
        Self::parse(reader, *kind, rest).await
    }

    pub(super) fn out_of_step() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, "unexpected Redis reply")
    }

    /// `None` for an error reply: the command failed, but the connection is still in step.
    pub(super) fn accepted(self) -> Option<Self> {
        (!self.error).then_some(self)
    }

    /// The bulk string, empty for nil.
    pub(super) fn into_bytes(self) -> Option<Vec<u8>> {
        self.bytes
    }

    pub(super) fn into_integer(self) -> Option<i64> {
        self.integer
    }

    async fn parse<R>(reader: &mut R, kind: u8, rest: &[u8]) -> io::Result<Self>
    where
        R: AsyncBufRead + Unpin,
    {
        match (kind, rest) {
            (b'+', _) => Ok(Self::default()),
            (b'-', _) => Ok(Self {
                error: true,
                ..Self::default()
            }),
            (b':', number) => Ok(Self {
                integer: Some(Self::number(number)?),
                ..Self::default()
            }),
            (b'$', b"-1") => Ok(Self::bulk(Vec::new())),
            (b'$', length) => Self::read_bulk(reader, length).await,
            _ => Err(Self::out_of_step()),
        }
    }

    async fn read_bulk<R>(reader: &mut R, length: &[u8]) -> io::Result<Self>
    where
        R: AsyncBufRead + Unpin,
    {
        let mut bytes = vec![0; Self::bulk_length(length)?];
        reader.read_exact(&mut bytes).await?;
        let rest = Self::line(reader).await?;
        rest.is_empty()
            .then_some(Self::bulk(bytes))
            .ok_or_else(Self::out_of_step)
    }

    fn bulk(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Some(bytes),
            ..Self::default()
        }
    }

    fn bulk_length(text: &[u8]) -> io::Result<usize> {
        usize::try_from(Self::number(text)?)
            .ok()
            .and_then(|length| (length <= Self::MAX_BULK).then_some(length))
            .ok_or_else(Self::out_of_step)
    }

    fn number(text: &[u8]) -> io::Result<i64> {
        std::str::from_utf8(text)
            .ok()
            .and_then(|text| text.parse().ok())
            .ok_or_else(Self::out_of_step)
    }

    /// One line without its `\r\n`, refused past `MAX_LINE` bytes.
    async fn line<R>(reader: &mut R) -> io::Result<Vec<u8>>
    where
        R: AsyncBufRead + Unpin,
    {
        let mut line = Vec::new();
        reader
            .take(Self::MAX_LINE)
            .read_until(b'\n', &mut line)
            .await?;
        line.strip_suffix(b"\r\n")
            .map(<[u8]>::to_vec)
            .ok_or_else(Self::out_of_step)
    }
}
