use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(super) struct EgressPump;

impl EgressPump {
    const IDLE: Duration = Duration::from_mins(5);
    const CHUNK: usize = 16 * 1024;

    pub(super) async fn exchange(
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
        let (mut client_read, mut client_write) = client.split();
        let (mut upstream_read, mut upstream_write) = upstream.split();
        let mut outgoing = vec![0_u8; Self::CHUNK];
        let mut incoming = vec![0_u8; Self::CHUNK];
        let mut open = [true, true];
        while open.contains(&true) {
            let step = async {
                tokio::select! {
                    read = client_read.read(&mut outgoing), if open[0] => {
                        let read = read?;
                        open[0] = read > 0;
                        Self::forward(&mut upstream_write, &outgoing[..read]).await
                    }
                    read = upstream_read.read(&mut incoming), if open[1] => {
                        let read = read?;
                        open[1] = read > 0;
                        Self::forward(&mut client_write, &incoming[..read]).await
                    }
                }
            };
            let Ok(result) = tokio::time::timeout(idle, step).await else {
                return Ok(());
            };
            result?;
        }
        Ok(())
    }

    async fn forward(
        writer: &mut (impl AsyncWriteExt + Unpin),
        bytes: &[u8],
    ) -> std::io::Result<()> {
        if bytes.is_empty() {
            return writer.shutdown().await;
        }
        writer.write_all(bytes).await
    }
}
