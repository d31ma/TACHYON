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
        if self.held.is_empty() {
            return;
        }
        let merged = Self::merged(&self.held);
        self.outgoing.extend_from_slice(&merged);
        self.held.clear();
    }

    fn packets(bytes: &[u8]) -> Option<Vec<(&[u8], &[u8])>> {
        let mut packets = Vec::new();
        let mut rest = bytes;
        while !rest.is_empty() {
            let header = rest.get(..Self::HEADER_BYTES)?;
            let length = usize::from(u16::from_be_bytes([header[2], header[3]]));
            let prelogin = header[0] == Self::PRELOGIN && length >= Self::HEADER_BYTES;
            let packet = rest.get(..length).filter(|_| prelogin)?;
            packets.push((header, &packet[Self::HEADER_BYTES..]));
            rest = &rest[length..];
        }
        Some(packets)
    }

    fn merged(held: &[u8]) -> Vec<u8> {
        let Some(packets) = Self::packets(held).filter(|packets| packets.len() > 1) else {
            return held.to_vec();
        };
        let first = packets[0].0;
        let payload: Vec<u8> = packets
            .iter()
            .flat_map(|(_, body)| body.iter().copied())
            .collect();
        let chunks: Vec<&[u8]> = payload.chunks(Self::PAYLOAD_LIMIT).collect();
        let mut merged = Vec::with_capacity(payload.len() + chunks.len() * Self::HEADER_BYTES);
        let mut packet_id = first[6];
        for (index, chunk) in chunks.iter().enumerate() {
            let last = index + 1 == chunks.len();
            let status = if last {
                first[1] | Self::END_OF_MESSAGE
            } else {
                first[1] & !Self::END_OF_MESSAGE
            };
            let length = u16::try_from(chunk.len() + Self::HEADER_BYTES).unwrap_or(u16::MAX);
            let [high, low] = length.to_be_bytes();
            merged.extend_from_slice(&[
                Self::PRELOGIN,
                status,
                high,
                low,
                first[4],
                first[5],
                packet_id,
                first[7],
            ]);
            merged.extend_from_slice(chunk);
            packet_id = packet_id.wrapping_add(1);
        }
        merged
    }

    fn holds(&self, bytes: &[u8]) -> bool {
        let at_packet_end = Self::packets(&self.held).is_some();
        !at_packet_end || bytes.first().is_none_or(|byte| *byte == Self::PRELOGIN)
    }
}

impl<S: AsyncWrite + Unpin> PreloginStream<S> {
    fn poll_send(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        while self.sent < self.outgoing.len() {
            let written =
                ready!(Pin::new(&mut self.inner).poll_write(context, &self.outgoing[self.sent..]))?;
            if written == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            }
            self.sent += written;
        }
        if !self.outgoing.is_empty() {
            self.outgoing.clear();
            self.sent = 0;
            self.flush_owed = true;
        }
        if self.flush_owed {
            ready!(Pin::new(&mut self.inner).poll_flush(context))?;
            self.flush_owed = false;
        }
        Poll::Ready(Ok(()))
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
        if !this.passing {
            if this.holds(buffer) {
                this.held.extend_from_slice(buffer);
                return Poll::Ready(Ok(buffer.len()));
            }
            this.passing = true;
            this.release();
        }
        ready!(this.poll_send(context))?;
        Pin::new(&mut this.inner).poll_write(context, buffer)
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
