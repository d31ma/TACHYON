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
        Self::until_idle(both, Self::idle_watch(started, &last_activity, idle)).await
    }

    async fn until_idle(
        both: impl Future<Output = std::io::Result<((), ())>>,
        idle: impl Future<Output = ()>,
    ) -> std::io::Result<()> {
        tokio::select! {
            result = both => result.map(drop),
            () = idle => Ok(()),
        }
    }

    async fn forward(
        mut reader: impl AsyncRead + Unpin,
        mut writer: impl AsyncWrite + Unpin,
        started: Instant,
        last_activity: &AtomicU64,
    ) -> std::io::Result<()> {
        let mut chunk = vec![0_u8; Self::CHUNK];
        let activity = (started, last_activity);
        while Self::step(&mut reader, &mut writer, &mut chunk, activity).await? {}
        Ok(())
    }

    async fn step(
        reader: &mut (impl AsyncRead + Unpin),
        writer: &mut (impl AsyncWrite + Unpin),
        chunk: &mut [u8],
        (started, last_activity): (Instant, &AtomicU64),
    ) -> std::io::Result<bool> {
        let read = reader.read(chunk).await?;
        Self::mark(started, last_activity);
        if read == 0 {
            writer.shutdown().await?;
            return Ok(false);
        }
        writer.write_all(&chunk[..read]).await?;
        Self::mark(started, last_activity);
        Ok(true)
    }

    async fn idle_watch(started: Instant, last_activity: &AtomicU64, idle: Duration) {
        while Self::still_active(started, last_activity, idle).await {}
    }

    async fn still_active(started: Instant, last_activity: &AtomicU64, idle: Duration) -> bool {
        let last = Duration::from_millis(last_activity.load(Ordering::Relaxed));
        let deadline = started + last + idle;
        let active = Instant::now() < deadline;
        tokio::time::sleep_until(deadline).await;
        active
    }

    fn mark(started: Instant, last_activity: &AtomicU64) {
        let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        last_activity.store(elapsed, Ordering::Relaxed);
    }
}
