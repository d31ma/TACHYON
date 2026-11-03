use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::io::{AsyncRead, AsyncWrite};

pub(super) struct PreloginStream<S> {
    inner: S,
    held: Vec<u8>,
    outgoing: Vec<u8>,
    sent: usize,
    flush_owed: bool,
    passing: bool,
}

impl<S> PreloginStream<S> {
    const HEADER_BYTES: usize = 8;
    const PRELOGIN: u8 = 0x12;
    const END_OF_MESSAGE: u8 = 0x01;
    const PAYLOAD_LIMIT: usize = 4088;

    pub(super) fn new(inner: S) -> Self {
        Self {
            inner,
            held: Vec::new(),
            outgoing: Vec::new(),
            sent: 0,
            flush_owed: false,
            passing: false,
        }
    }

    fn release(&mut self) {
        let merged = Self::merged(&self.held);
        self.outgoing.extend_from_slice(&merged);
        self.held.clear();
    }

    fn next_packet(rest: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
        let header = rest.get(..Self::HEADER_BYTES)?;
        let length = usize::from(u16::from_be_bytes([header[2], header[3]]));
        let prelogin = [header[0] == Self::PRELOGIN, length >= Self::HEADER_BYTES] == [true, true];
        let packet = rest.get(..length).filter(|_| prelogin)?;
        Some((header, &packet[Self::HEADER_BYTES..], &rest[length..]))
    }

    fn packets(bytes: &[u8]) -> Option<Vec<(&[u8], &[u8])>> {
        let mut packets = Vec::new();
        let mut rest = bytes;
        while !rest.is_empty() {
            let (header, body, after) = Self::next_packet(rest)?;
            packets.push((header, body));
            rest = after;
        }
        Some(packets)
    }

    fn merged(held: &[u8]) -> Vec<u8> {
        Self::packets(held)
            .filter(|packets| packets.len() > 1)
            .map_or_else(|| held.to_vec(), |packets| Self::joined(&packets))
    }

    fn joined(packets: &[(&[u8], &[u8])]) -> Vec<u8> {
        let first = packets[0].0;
        let payload: Vec<u8> = packets
            .iter()
            .flat_map(|(_, body)| body.iter().copied())
            .collect();
        let chunks: Vec<&[u8]> = payload.chunks(Self::PAYLOAD_LIMIT).collect();
        let mut merged = Vec::with_capacity(payload.len() + chunks.len() * Self::HEADER_BYTES);
        for (index, chunk) in chunks.iter().enumerate() {
            let last = index + 1 == chunks.len();
            merged.extend_from_slice(&Self::header(first, chunk.len(), index, last));
            merged.extend_from_slice(chunk);
        }
        merged
    }

    fn header(first: &[u8], body: usize, index: usize, last: bool) -> [u8; 8] {
        let status = (first[1] & !Self::END_OF_MESSAGE) | (Self::END_OF_MESSAGE * u8::from(last));
        let length = u16::try_from(body + Self::HEADER_BYTES).unwrap_or(u16::MAX);
        let [high, low] = length.to_be_bytes();
        let packet_id = first[6].wrapping_add(index.to_le_bytes()[0]);
        [
            Self::PRELOGIN,
            status,
            high,
            low,
            first[4],
            first[5],
            packet_id,
            first[7],
        ]
    }

    fn holds(&self, bytes: &[u8]) -> bool {
        let at_packet_end = Self::packets(&self.held).is_some();
        let next_is_prelogin = bytes.first().is_none_or(|byte| *byte == Self::PRELOGIN);
        [at_packet_end, next_is_prelogin] != [true, false]
    }

    fn hold(&mut self, buffer: &[u8]) -> Option<usize> {
        let keep = [self.passing, self.holds(buffer)] == [false, true];
        self.passing = !keep;
        if keep {
            self.held.extend_from_slice(buffer);
            return Some(buffer.len());
        }
        self.release();
        None
    }

    fn written(written: usize) -> io::Result<usize> {
        (written > 0)
            .then_some(written)
            .ok_or_else(|| io::ErrorKind::WriteZero.into())
    }
}

impl<S: AsyncWrite + Unpin> PreloginStream<S> {
    fn poll_outgoing(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        while self.sent < self.outgoing.len() {
            let written =
                ready!(Pin::new(&mut self.inner).poll_write(context, &self.outgoing[self.sent..]))?;
            self.sent += Self::written(written)?;
        }
        self.flush_owed |= !self.outgoing.is_empty();
        self.outgoing.clear();
        self.sent = 0;
        Poll::Ready(Ok(()))
    }

    fn poll_owed_flush(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.flush_owed {
            ready!(Pin::new(&mut self.inner).poll_flush(context))?;
            self.flush_owed = false;
        }
        Poll::Ready(Ok(()))
    }

    fn poll_send(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(self.poll_outgoing(context))?;
        self.poll_owed_flush(context)
    }

    fn poll_write_through(
        &mut self,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        ready!(self.poll_send(context))?;
        Pin::new(&mut self.inner).poll_write(context, buffer)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for PreloginStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        this.release();
        ready!(this.poll_send(context))?;
        Pin::new(&mut this.inner).poll_read(context, buffer)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for PreloginStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let held = this.hold(buffer);
        held.map_or_else(
            || this.poll_write_through(context, buffer),
            |length| Poll::Ready(Ok(length)),
        )
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_send(context))?;
        Pin::new(&mut this.inner).poll_flush(context)
    }

    fn poll_close(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.release();
        ready!(this.poll_send(context))?;
        Pin::new(&mut this.inner).poll_close(context)
    }
}

#[cfg(test)]
mod tests;
