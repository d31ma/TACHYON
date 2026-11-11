use super::{EgressActivity, EgressAnswer, EgressFraming, EgressRequest};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

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
        let activity = EgressActivity::new();
        let (client_read, client_write) = client.split();
        let (upstream_read, upstream_write) = upstream.split();
        let both = async {
            tokio::try_join!(
                Self::forward(client_read, upstream_write, &activity),
                Self::forward(upstream_read, client_write, &activity),
            )
        };
        Self::until_idle(both, activity.until_idle(idle)).await
    }

    pub(super) async fn exchange_once(
        client: &mut TcpStream,
        upstream: &mut TcpStream,
        early: Vec<u8>,
        framing: EgressFraming,
    ) -> std::io::Result<()> {
        let activity = EgressActivity::new();
        let (client_read, client_write) = client.split();
        let (upstream_read, upstream_write) = upstream.split();
        let both = async {
            tokio::try_join!(
                EgressRequest::send(client_read, upstream_write, early, framing, &activity),
                Self::answer(upstream_read, client_write, &activity),
            )
        };
        Self::until_idle(both, activity.until_idle(Self::IDLE)).await
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

    async fn answer(
        mut reader: impl AsyncRead + Unpin,
        mut writer: impl AsyncWrite + Unpin,
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        let rest = EgressAnswer::relay_heads(&mut reader, &mut writer).await?;
        writer.write_all(&rest).await?;
        activity.mark();
        Self::forward(reader, writer, activity).await
    }

    async fn forward(
        mut reader: impl AsyncRead + Unpin,
        mut writer: impl AsyncWrite + Unpin,
        activity: &EgressActivity,
    ) -> std::io::Result<()> {
        let mut chunk = vec![0_u8; Self::CHUNK];
        while Self::step(&mut reader, &mut writer, &mut chunk, activity).await? {}
        Ok(())
    }

    async fn step(
        reader: &mut (impl AsyncRead + Unpin),
        writer: &mut (impl AsyncWrite + Unpin),
        chunk: &mut [u8],
        activity: &EgressActivity,
    ) -> std::io::Result<bool> {
        let read = reader.read(chunk).await?;
        activity.mark();
        if read == 0 {
            writer.shutdown().await?;
            return Ok(false);
        }
        writer.write_all(&chunk[..read]).await?;
        activity.mark();
        Ok(true)
    }
}
