#![allow(clippy::expect_used)]

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::PreloginStream;

#[derive(Default)]
struct Wire {
    written: Vec<u8>,
    flushes: usize,
}

impl AsyncRead for Wire {
    fn poll_read(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        buffer[0] = 1;
        Poll::Ready(Ok(1))
    }
}

impl AsyncWrite for Wire {
    fn poll_write(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.get_mut().written.extend_from_slice(buffer);
        Poll::Ready(Ok(buffer.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().flushes += 1;
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn packet(status: u8, packet_id: u8, body: &[u8]) -> Vec<u8> {
    let [high, low] = u16::try_from(body.len() + 8)
        .expect("small packet")
        .to_be_bytes();
    let mut bytes = vec![0x12, status, high, low, 0, 0, packet_id, 0];
    bytes.extend_from_slice(body);
    bytes
}

async fn send_each(stream: &mut PreloginStream<Wire>, packets: &[Vec<u8>]) {
    for bytes in packets {
        stream.write_all(bytes).await.expect("write");
        stream.flush().await.expect("flush");
    }
}

async fn read_one(stream: &mut PreloginStream<Wire>) {
    let mut answer = [0u8; 1];
    stream.read_exact(&mut answer).await.expect("read");
}

#[tokio::test]
async fn a_handshake_flight_flushed_record_by_record_reaches_the_server_as_one_message() {
    let mut stream = PreloginStream::new(Wire::default());
    let records = [b"key-exchange".as_slice(), b"cipher-spec", b"finished"];
    send_each(&mut stream, &records.map(|record| packet(1, 1, record))).await;
    assert!(
        stream.inner.written.is_empty(),
        "nothing leaves before the client reads"
    );
    read_one(&mut stream).await;
    assert_eq!(stream.inner.written, packet(1, 1, &records.concat()));
    assert!(stream.inner.flushes > 0);
}

#[tokio::test]
async fn one_packet_and_bytes_after_the_handshake_pass_unchanged_and_in_order() {
    let mut stream = PreloginStream::new(Wire::default());
    let hello = packet(1, 1, b"client-hello");
    send_each(&mut stream, std::slice::from_ref(&hello)).await;
    read_one(&mut stream).await;
    let last_flight = packet(1, 1, b"finished");
    let record = [0x17, 3, 3, 0, 2, 0xAA, 0xBB];
    stream.write_all(&last_flight).await.expect("write");
    stream.write_all(&record).await.expect("write");
    stream
        .write_all(&packet(1, 1, b"later"))
        .await
        .expect("write");
    stream.flush().await.expect("flush");
    let mut expected = hello;
    expected.extend_from_slice(&last_flight);
    expected.extend_from_slice(&record);
    expected.extend_from_slice(&packet(1, 1, b"later"));
    assert_eq!(stream.inner.written, expected);
}

#[tokio::test]
async fn a_merged_flight_longer_than_one_packet_ends_only_its_last_packet() {
    let mut stream = PreloginStream::new(Wire::default());
    let first = vec![7u8; 3000];
    let second = vec![9u8; 3000];
    send_each(&mut stream, &[packet(1, 1, &first), packet(1, 1, &second)]).await;
    read_one(&mut stream).await;
    let payload = [first, second].concat();
    let mut expected = packet(0, 1, &payload[..4088]);
    expected.extend_from_slice(&packet(1, 2, &payload[4088..]));
    assert_eq!(stream.inner.written, expected);
}

#[tokio::test]
async fn bytes_that_are_not_whole_prelogin_packets_pass_unchanged() {
    let mut stream = PreloginStream::new(Wire::default());
    let mut cut = packet(1, 1, b"client-hello");
    cut.truncate(10);
    stream.write_all(&cut).await.expect("write");
    read_one(&mut stream).await;
    assert_eq!(stream.inner.written, cut);
}
