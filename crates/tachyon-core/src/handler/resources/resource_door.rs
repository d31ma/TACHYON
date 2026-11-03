use super::DoorRoute;
use crate::Failure;
use crate::failure::diagnostic;
use futures_util::future::OptionFuture;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug)]
pub(in crate::handler) struct ResourceDoor {
    address: SocketAddr,
    routes: Arc<Mutex<HashMap<[u8; 32], SocketAddr>>>,
}

impl ResourceDoor {
    const MAX_CONNECTIONS: usize = 256;
    const MAX_HEAD_BYTES: usize = 8192;
    const READ_CHUNK: usize = 1024;
    const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
    const ACCEPT_PAUSE: Duration = Duration::from_millis(100);
    const HEAD_END: &'static [u8] = b"\r\n\r\n";
    const AUTHORIZATION: &'static str = "authorization:";
    const BEARER: &'static str = "Bearer ";
    const UNKNOWN: &'static [u8] =
        b"HTTP/1.1 401 Unauthorized\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
    const BUSY: &'static [u8] =
        b"HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

    pub(in crate::handler) async fn open() -> Result<Arc<Self>, Failure> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|error| Self::unavailable(&error))?;
        let address = listener
            .local_addr()
            .map_err(|error| Self::unavailable(&error))?;
        let routes = Arc::default();
        tokio::spawn(Self::accept(listener, Arc::clone(&routes)));
        Ok(Arc::new(Self { address, routes }))
    }

    pub(in crate::handler) const fn address(&self) -> SocketAddr {
        self.address
    }

    pub(in crate::handler) const fn port(&self) -> u16 {
        self.address.port()
    }

    pub(super) fn register(&self, token: &str, listener: SocketAddr) -> DoorRoute {
        let key = Self::key(token);
        let _registered = self
            .routes
            .lock()
            .map(|mut routes| routes.insert(key, listener));
        DoorRoute::new(Arc::clone(&self.routes), key)
    }

    fn key(token: &str) -> [u8; 32] {
        Sha256::digest(token.as_bytes()).into()
    }

    async fn accept(listener: TcpListener, routes: Arc<Mutex<HashMap<[u8; 32], SocketAddr>>>) {
        let slots = Arc::new(Semaphore::new(Self::MAX_CONNECTIONS));
        loop {
            Self::take(listener.accept().await, &routes, &slots).await;
        }
    }

    async fn take(
        accepted: std::io::Result<(TcpStream, SocketAddr)>,
        routes: &Arc<Mutex<HashMap<[u8; 32], SocketAddr>>>,
        slots: &Arc<Semaphore>,
    ) {
        match accepted {
            Ok((stream, _)) => Self::admit(stream, routes, slots),
            Err(_) => tokio::time::sleep(Self::ACCEPT_PAUSE).await,
        }
    }

    fn admit(
        stream: TcpStream,
        routes: &Arc<Mutex<HashMap<[u8; 32], SocketAddr>>>,
        slots: &Arc<Semaphore>,
    ) {
        let slot = Arc::clone(slots).try_acquire_owned().ok();
        tokio::spawn(Self::serve(stream, Arc::clone(routes), slot));
    }

    async fn serve(
        mut client: TcpStream,
        routes: Arc<Mutex<HashMap<[u8; 32], SocketAddr>>>,
        slot: Option<OwnedSemaphorePermit>,
    ) -> std::io::Result<()> {
        let answer = [Self::UNKNOWN, Self::BUSY][usize::from(slot.is_none())];
        let head = OptionFuture::from(slot.as_ref().map(|_| Self::read_head_in_time(&mut client)))
            .await
            .flatten();
        let target = head
            .as_deref()
            .and_then(Self::token)
            .and_then(|token| Self::target(&routes, token));
        let (Some(head), Some(target)) = (head, target) else {
            return client.write_all(answer).await;
        };
        let mut upstream = TcpStream::connect(target).await?;
        upstream.write_all(&head).await?;
        let exchanged = crate::handler::EgressPump::exchange(&mut client, &mut upstream).await;
        drop(slot);
        exchanged
    }

    async fn read_head_in_time(client: &mut TcpStream) -> Option<Vec<u8>> {
        tokio::time::timeout(Self::HEAD_TIMEOUT, Self::read_head(client))
            .await
            .ok()
            .flatten()
    }

    async fn read_head(client: &mut TcpStream) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut chunk = vec![0_u8; Self::READ_CHUNK];
        while !Self::has_head(&bytes) {
            let read = client
                .read(&mut chunk)
                .await
                .ok()
                .filter(|read| *read > 0)?;
            bytes.extend_from_slice(&chunk[..read]);
            (bytes.len() <= Self::MAX_HEAD_BYTES + Self::READ_CHUNK).then_some(())?;
        }
        Some(bytes)
    }

    fn has_head(bytes: &[u8]) -> bool {
        bytes
            .windows(Self::HEAD_END.len())
            .any(|window| window == Self::HEAD_END)
    }

    fn token(head: &[u8]) -> Option<&str> {
        let text = std::str::from_utf8(head).ok()?;
        text.split("\r\n")
            .take_while(|line| !line.is_empty())
            .find_map(|line| {
                let lowered = line.get(..Self::AUTHORIZATION.len())?.to_ascii_lowercase();
                (lowered == Self::AUTHORIZATION).then(|| line[Self::AUTHORIZATION.len()..].trim())
            })?
            .strip_prefix(Self::BEARER)
    }

    fn target(routes: &Mutex<HashMap<[u8; 32], SocketAddr>>, token: &str) -> Option<SocketAddr> {
        let routes = routes.lock().ok()?;
        routes.get(&Self::key(token)).copied()
    }

    fn unavailable(error: &std::io::Error) -> Failure {
        Failure::one(diagnostic(
            2010,
            format!("Cannot open the Yon managed resource door: {error}"),
            None,
            None,
        ))
    }
}
