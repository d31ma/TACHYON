use super::ResourceAccess;
use super::resource_request::ResourceRequest;
use crate::handler::gateway::GatewayCalls;
use crate::handler::process::HandlerCancellation;
use crate::handler::topic_bus::TopicOutbox;
use crate::storage::ManagedStorage;
use axum::http::{HeaderMap, StatusCode, header};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{JoinError, JoinHandle, JoinSet};
use tokio::time::{Instant, sleep_until};

#[derive(Clone)]
pub(super) struct Access {
    pub(super) storage: Option<Arc<ManagedStorage>>,
    pub(super) gateways: Option<Arc<GatewayCalls>>,
    pub(super) topics: Option<Arc<TopicOutbox>>,
    pub(super) token: String,
    pub(super) host: String,
    pub(super) deadline: Instant,
    pub(super) stop: HandlerCancellation,
}

impl Access {
    const HEAD_END: &'static [u8] = b"\r\n\r\n";
    const FAILED_BODY: &'static [u8] = br#"{"error":"Yon managed resource operation failed."}"#;

    pub(super) fn spawn(self, listener: TcpListener) -> JoinHandle<()> {
        tokio::spawn(self.serve(listener))
    }

    async fn serve(self, listener: TcpListener) {
        let mut connections = JoinSet::new();
        while self.accept_next(&listener, &mut connections).await {}
        self.stop.cancel();
        drop(listener);
        Self::finish_connections(connections).await;
    }

    async fn accept_next(&self, listener: &TcpListener, connections: &mut JoinSet<()>) -> bool {
        tokio::select! {
            biased;
            () = self.stop.cancelled() => false,
            () = sleep_until(self.deadline) => false,
            accepted = listener.accept(), if connections.len() < ResourceAccess::MAX_CONNECTIONS => {
                self.spawn_connection(accepted, connections)
            }
            _settled = connections.join_next(), if !connections.is_empty() => true,
        }
    }

    fn spawn_connection(
        &self,
        accepted: std::io::Result<(TcpStream, SocketAddr)>,
        connections: &mut JoinSet<()>,
    ) -> bool {
        accepted
            .map(|(stream, _)| self.spawn_execute(stream, connections))
            .is_ok()
    }

    fn spawn_execute(&self, stream: TcpStream, connections: &mut JoinSet<()>) {
        let access = self.clone();
        connections.spawn(async move {
            let _result = access.execute(stream).await;
        });
    }

    async fn finish_connections(mut connections: JoinSet<()>) {
        while connections.join_next().await.is_some() {}
    }

    async fn bounded<T>(&self, operation: impl Future<Output = T>) -> Result<T, StatusCode> {
        tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(StatusCode::REQUEST_TIMEOUT),
            () = sleep_until(self.deadline) => Err(StatusCode::REQUEST_TIMEOUT),
            result = operation => Ok(result),
        }
    }

    async fn execute(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let result = self.operation(&mut stream).await;
        self.bounded(Self::write_response(&mut stream, result))
            .await
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::TimedOut))?
    }

    async fn write_response(
        stream: &mut TcpStream,
        result: Result<Vec<u8>, StatusCode>,
    ) -> std::io::Result<()> {
        let (status, bytes) = match result {
            Ok(bytes) => (StatusCode::OK, bytes),
            Err(status) => (status, Self::FAILED_BODY.to_vec()),
        };
        let headers = format!(
            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            status,
            bytes.len()
        );
        stream.write_all(headers.as_bytes()).await?;
        stream.write_all(&bytes).await?;
        stream.shutdown().await
    }

    async fn operation(&self, stream: &mut TcpStream) -> Result<Vec<u8>, StatusCode> {
        let command = self.bounded(self.read_command(stream)).await??;
        let kind = command
            .get("operation")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        self.run_operation(&kind, command).await
    }

    async fn run_operation(&self, kind: &str, command: Value) -> Result<Vec<u8>, StatusCode> {
        match kind {
            "gateway" => self.answer_gateway(&command).await,
            "topic" => self.publish_topic(command).await,
            _ => self.execute_storage(&command).await,
        }
    }

    async fn answer_gateway(&self, command: &Value) -> Result<Vec<u8>, StatusCode> {
        let gateways = self.gateways.as_ref().ok_or(StatusCode::BAD_REQUEST)?;
        Ok(gateways.answer(command).await)
    }

    async fn publish_topic(&self, command: Value) -> Result<Vec<u8>, StatusCode> {
        let topics = Arc::clone(self.topics.as_ref().ok_or(StatusCode::BAD_REQUEST)?);
        let published = tokio::task::spawn_blocking(move || topics.publish(&command));
        Self::published_answer(&self.bounded(published).await?)
    }

    fn published_answer(
        published: &Result<Result<(), &'static str>, JoinError>,
    ) -> Result<Vec<u8>, StatusCode> {
        match published {
            Ok(Ok(())) => Ok(b"{}".to_vec()),
            _ => Err(StatusCode::BAD_REQUEST),
        }
    }

    async fn execute_storage(&self, command: &Value) -> Result<Vec<u8>, StatusCode> {
        let storage = self.storage.as_ref().ok_or(StatusCode::BAD_REQUEST)?;
        let result = self
            .bounded(storage.execute(command))
            .await?
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let bytes = serde_json::to_vec(&result).map_err(|_| StatusCode::BAD_GATEWAY)?;
        Self::check_answer_size(&bytes)?;
        Ok(bytes)
    }

    fn check_answer_size(bytes: &[u8]) -> Result<(), StatusCode> {
        if bytes.len() > ResourceAccess::MAX_BYTES {
            return Err(StatusCode::BAD_GATEWAY);
        }
        Ok(())
    }

    async fn read_command(&self, stream: &mut TcpStream) -> Result<Value, StatusCode> {
        let head = Self::read_request_head(stream).await?;
        let (headers, length) = ResourceRequest::headers(&head)?;
        self.check_authorized(&headers)?;
        let mut body = vec![0; length];
        stream
            .read_exact(&mut body)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)
    }

    async fn read_request_head(stream: &mut TcpStream) -> Result<Vec<u8>, StatusCode> {
        let mut head = Vec::new();
        while !head.ends_with(Self::HEAD_END) {
            Self::read_head_byte(stream, &mut head).await?;
        }
        Ok(head)
    }

    async fn read_head_byte(stream: &mut TcpStream, head: &mut Vec<u8>) -> Result<(), StatusCode> {
        if head.len() == ResourceAccess::MAX_HEAD_BYTES {
            return Err(StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
        }
        let byte = stream
            .read_u8()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        head.push(byte);
        Ok(())
    }

    fn check_authorized(&self, headers: &HeaderMap) -> Result<(), StatusCode> {
        if !self.authorized(headers) {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(())
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        let supplied = headers
            .get(header::AUTHORIZATION)
            .map_or(&[][..], |value| value.as_bytes());
        let expected = format!("Bearer {}", self.token);
        let checks = [
            Self::constant_time_equal(supplied, expected.as_bytes()),
            !headers.contains_key(header::ORIGIN),
            headers
                .get(header::HOST)
                .is_some_and(|host| host.as_bytes() == self.host.as_bytes()),
            headers
                .get(header::CONTENT_TYPE)
                .is_some_and(|kind| kind == "application/json"),
        ];
        !checks.contains(&false)
    }

    fn constant_time_equal(supplied: &[u8], expected: &[u8]) -> bool {
        supplied.len() == expected.len() && Self::byte_difference(supplied, expected) == 0
    }

    fn byte_difference(supplied: &[u8], expected: &[u8]) -> u8 {
        supplied
            .iter()
            .zip(expected)
            .fold(0_u8, |difference, (supplied_byte, expected_byte)| {
                difference | (supplied_byte ^ expected_byte)
            })
    }
}
