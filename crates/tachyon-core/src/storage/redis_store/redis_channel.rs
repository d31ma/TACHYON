use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, mpsc, oneshot};

use super::RedisReply;

type Request = (Vec<u8>, oneshot::Sender<RedisReply>);
type Waiting = Arc<Mutex<VecDeque<oneshot::Sender<RedisReply>>>>;

#[derive(Clone)]
pub(super) struct RedisChannel {
    requests: mpsc::Sender<Request>,
}

impl RedisChannel {
    const RESPONSE_TIMEOUT: Duration = Duration::from_millis(500);
    const QUEUE: usize = 50;

    pub(super) fn spawn<S>(stream: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        let (requests, queue) = mpsc::channel(Self::QUEUE);
        tokio::spawn(Self::drive(stream, queue));
        Self { requests }
    }

    pub(super) fn is_closed(&self) -> bool {
        self.requests.is_closed()
    }

    pub(super) async fn call(&self, arguments: &[&[u8]]) -> Option<RedisReply> {
        tokio::time::timeout(
            Self::RESPONSE_TIMEOUT,
            self.exchange(Self::encode(arguments)),
        )
        .await
        .ok()
        .flatten()?
        .accepted()
    }

    async fn exchange(&self, bytes: Vec<u8>) -> Option<RedisReply> {
        let (reply, answer) = oneshot::channel();
        self.requests.send((bytes, reply)).await.ok()?;
        answer.await.ok()
    }

    fn encode(arguments: &[&[u8]]) -> Vec<u8> {
        let mut bytes = format!("*{}\r\n", arguments.len()).into_bytes();
        for argument in arguments {
            bytes.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
            bytes.extend_from_slice(argument);
            bytes.extend_from_slice(b"\r\n");
        }
        bytes
    }

    async fn drive<S>(stream: S, queue: mpsc::Receiver<Request>)
    where
        S: AsyncRead + AsyncWrite + Send,
    {
        let (reader, writer) = tokio::io::split(stream);
        let waiting = Waiting::default();
        let outgoing = std::pin::pin!(Self::write(writer, queue, Arc::clone(&waiting)));
        let incoming = std::pin::pin!(Self::read(BufReader::new(reader), waiting));
        future::select(outgoing, incoming).await;
    }

    async fn write<W>(
        mut writer: W,
        mut queue: mpsc::Receiver<Request>,
        waiting: Waiting,
    ) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        while let Some((bytes, reply)) = queue.recv().await {
            waiting.lock().await.push_back(reply);
            writer.write_all(&bytes).await?;
            writer.flush().await?;
        }
        Ok(())
    }

    async fn read<R>(mut reader: R, waiting: Waiting) -> io::Result<()>
    where
        R: AsyncBufRead + Unpin,
    {
        loop {
            let reply = RedisReply::read(&mut reader).await?;
            let command = waiting
                .lock()
                .await
                .pop_front()
                .ok_or_else(RedisReply::out_of_step)?;
            command.send(reply).ok();
        }
    }
}
