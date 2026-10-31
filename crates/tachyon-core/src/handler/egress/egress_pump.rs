use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Instant;

pub(crate) struct EgressPump;

impl EgressPump {
    const IDLE: Duration = Duration::from_mins(5);
    const CHUNK: usize = 16 * 1024;

    pub(crate) async fn exchange(
        client: &mut TcpStream,
        upstream: &mut TcpStream,
    ) -> std::io::Result<()> {
        Self::exchange_until_idle(client, upstream, Self::IDLE).await
    }

    pub(super) async fn exchange_until_idle(
        client: &mut TcpStream,
        upstream: &mut TcpStream,
        idle: Duration,
    ) -> std::io::Result<()> {
        let started = Instant::now();
        let last_activity = AtomicU64::new(0);
        let (client_read, client_write) = client.split();
        let (upstream_read, upstream_write) = upstream.split();
        let both = async {
            tokio::try_join!(
                Self::forward(client_read, upstream_write, started, &last_activity),
                Self::forward(upstream_read, client_write, started, &last_activity),
            )
        };
        tokio::select! {
            result = both => result.map(drop),
            () = Self::idle_watch(started, &last_activity, idle) => Ok(()),
        }
    }

    async fn forward(
        mut reader: impl AsyncRead + Unpin,
        mut writer: impl AsyncWrite + Unpin,
        started: Instant,
        last_activity: &AtomicU64,
    ) -> std::io::Result<()> {
        let mut chunk = vec![0_u8; Self::CHUNK];
        loop {
            let read = reader.read(&mut chunk).await?;
            Self::mark(started, last_activity);
            if read == 0 {
                return writer.shutdown().await;
            }
            writer.write_all(&chunk[..read]).await?;
            Self::mark(started, last_activity);
        }
    }

    async fn idle_watch(started: Instant, last_activity: &AtomicU64, idle: Duration) {
        loop {
            let last = Duration::from_millis(last_activity.load(Ordering::Relaxed));
            let deadline = started + last + idle;
            if Instant::now() >= deadline {
                return;
            }
            tokio::time::sleep_until(deadline).await;
        }
    }

    fn mark(started: Instant, last_activity: &AtomicU64) {
        let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        last_activity.store(elapsed, Ordering::Relaxed);
    }
}
