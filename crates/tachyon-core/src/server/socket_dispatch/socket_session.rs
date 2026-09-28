use super::SocketSeat;
use crate::Failure;
use crate::handler::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, SocketChannels, SocketInput,
    SocketOutput,
};
use crate::routing::RouteMatch;
use crate::server::{Dispatch, DispatchRoute, InvocationLog, ProtocolRequest, ServerResponse};
use axum::body::Body;
use axum::extract::FromRequestParts as _;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::http::header::{HOST, ORIGIN, UPGRADE};
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tokio::sync::mpsc;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::error::Elapsed;
use tokio::time::{Instant, sleep_until, timeout};
use tokio_util::sync::CancellationToken;

type HeldSocket = Result<Result<(), Failure>, JoinError>;

pub(in crate::server) struct SocketSession {
    supervisor: Arc<HandlerSupervisor>,
    handler: HandlerSource,
    mapping: Arc<crate::yon::plan::Route>,
    request: HandlerRequest,
    seat: SocketSeat,
    cancellation: CancellationToken,
}

impl SocketSession {
    const MAX_MESSAGE_BYTES: usize = SocketOutput::MAX_MESSAGE_BYTES;
    const MAX_FRAME_BYTES: usize = 1024 * 1024;
    const IDLE: Duration = Duration::from_mins(5);
    const LIFETIME: Duration = Duration::from_hours(1);
    const CLOSE_GRACE: Duration = Duration::from_secs(5);
    const QUEUE: usize = 16;
    const MAX_REASON_BYTES: usize = 120;

    pub(in crate::server) async fn open(
        state: &Dispatch,
        entry: &DispatchRoute,
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let (mut parts, _body) = request.into_parts();
        let upgrade = WebSocketUpgrade::from_request_parts(&mut parts, &())
            .await
            .ok();
        let allowed = Self::same_origin(&parts.headers);
        Self::answer(upgrade, allowed, &parts.headers, state, entry, matched)
    }

    fn answer(
        upgrade: Option<WebSocketUpgrade>,
        allowed: bool,
        headers: &HeaderMap,
        state: &Dispatch,
        entry: &DispatchRoute,
        matched: RouteMatch,
    ) -> Response<Body> {
        match (upgrade, allowed) {
            (None, _) => Self::upgrade_required(),
            (Some(_), false) => ServerResponse::text(
                StatusCode::FORBIDDEN,
                "This socket only accepts pages from its own origin.",
            ),
            (Some(upgrade), true) => Self::join(upgrade, headers, state, entry, matched),
        }
    }

    fn join(
        upgrade: WebSocketUpgrade,
        headers: &HeaderMap,
        state: &Dispatch,
        entry: &DispatchRoute,
        matched: RouteMatch,
    ) -> Response<Body> {
        state.sockets.join(&matched.route).map_or_else(
            || {
                ServerResponse::text(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Too many sockets are open. Try again later.",
                )
            },
            |(seat, peers)| Self::start(upgrade, seat, peers, headers, state, entry, matched),
        )
    }

    fn start(
        upgrade: WebSocketUpgrade,
        seat: SocketSeat,
        peers: mpsc::Receiver<String>,
        headers: &HeaderMap,
        state: &Dispatch,
        entry: &DispatchRoute,
        matched: RouteMatch,
    ) -> Response<Body> {
        let mut request = HandlerRequest::route(seat.id.clone(), &matched.route, HttpMethod::Get);
        request.operation = String::from("socket.open");
        request.parameters = matched.parameters;
        ProtocolRequest::copy_headers(&mut request, headers);
        let session = Self {
            supervisor: Arc::clone(&state.supervisor),
            handler: entry.handler.clone(),
            mapping: Arc::clone(&entry.mapping),
            request,
            seat,
            cancellation: state.cancellation.clone(),
        };
        upgrade
            .max_message_size(Self::MAX_FRAME_BYTES)
            .max_frame_size(Self::MAX_FRAME_BYTES)
            .on_upgrade(move |socket| session.run(socket, peers))
    }

    fn upgrade_required() -> Response<Body> {
        let mut response = ServerResponse::text(
            StatusCode::UPGRADE_REQUIRED,
            "This path is a WebSocket. Open it with new WebSocket(url) or a @SocketRequest client.",
        );
        response
            .headers_mut()
            .insert(UPGRADE, HeaderValue::from_static("websocket"));
        response
    }

    fn same_origin(headers: &HeaderMap) -> bool {
        headers
            .get(ORIGIN)
            .is_none_or(|origin| Self::origin_matches_host(origin, headers))
    }

    fn origin_matches_host(origin: &HeaderValue, headers: &HeaderMap) -> bool {
        let origin = origin
            .to_str()
            .ok()
            .and_then(|text| url::Url::parse(text).ok());
        let host = headers.get(HOST).and_then(|value| value.to_str().ok());
        origin
            .zip(host)
            .is_some_and(|(origin, host)| Self::authority_matches(&origin, host))
    }

    fn authority_matches(origin: &url::Url, host: &str) -> bool {
        origin.host_str().is_some_and(|origin_host| {
            Self::authority(origin_host, origin.port()).eq_ignore_ascii_case(host)
        })
    }

    fn authority(origin_host: &str, port: Option<u16>) -> String {
        port.map_or_else(
            || origin_host.to_owned(),
            |port| format!("{origin_host}:{port}"),
        )
    }

    async fn run(self, mut socket: WebSocket, mut peers: mpsc::Receiver<String>) {
        let (input, inputs) = mpsc::channel(Self::QUEUE);
        let (outputs_sender, mut outputs) = mpsc::channel(Self::QUEUE);
        let stop = HandlerCancellation::default();
        let mut process = tokio::spawn(Self::hold(
            Arc::clone(&self.supervisor),
            self.handler.clone(),
            self.request.clone(),
            SocketChannels {
                input: inputs,
                output: outputs_sender,
            },
            stop.clone(),
        ));
        let close = self
            .talk(&mut socket, &input, &mut outputs, &mut peers)
            .await;
        let _sent = socket.send(Message::Close(Some(close))).await;
        let _closed = timeout(Self::CLOSE_GRACE, input.send(SocketInput::Close)).await;
        drop(input);
        let finished = timeout(
            Self::CLOSE_GRACE,
            Self::drain_then_join(&mut outputs, &mut process),
        )
        .await;
        let result = Self::finished_or_stopped(finished, &stop, process).await;
        self.log_failure(result);
    }

    async fn drain_then_join(
        outputs: &mut mpsc::Receiver<SocketOutput>,
        process: &mut JoinHandle<Result<(), Failure>>,
    ) -> HeldSocket {
        while outputs.recv().await.is_some() {}
        process.await
    }

    async fn finished_or_stopped(
        finished: Result<HeldSocket, Elapsed>,
        stop: &HandlerCancellation,
        process: JoinHandle<Result<(), Failure>>,
    ) -> HeldSocket {
        if let Ok(result) = finished {
            return result;
        }
        stop.cancel();
        process.await
    }

    fn log_failure(&self, result: HeldSocket) {
        if let Ok(Err(failure)) = result
            && !self.cancellation.is_cancelled()
        {
            InvocationLog::log_failure(
                "handler.socket",
                &self.request.request_id,
                InvocationLog::runtime_family(&self.handler),
                &failure,
            );
        }
    }

    async fn hold(
        supervisor: Arc<HandlerSupervisor>,
        handler: HandlerSource,
        request: HandlerRequest,
        channels: SocketChannels,
        stop: HandlerCancellation,
    ) -> Result<(), Failure> {
        supervisor
            .invoke_socket(&handler, &request, channels, &stop, Self::LIFETIME)
            .await
    }

    async fn talk(
        &self,
        socket: &mut WebSocket,
        input: &mpsc::Sender<SocketInput>,
        outputs: &mut mpsc::Receiver<SocketOutput>,
        peers: &mut mpsc::Receiver<String>,
    ) -> CloseFrame {
        let mut idle = Instant::now() + Self::IDLE;
        let mut close = None;
        while close.is_none() {
            close = self
                .next_turn(socket, input, outputs, peers, &mut idle)
                .await;
        }
        close.unwrap_or_else(|| Self::close(1000, ""))
    }

    async fn next_turn(
        &self,
        socket: &mut WebSocket,
        input: &mpsc::Sender<SocketInput>,
        outputs: &mut mpsc::Receiver<SocketOutput>,
        peers: &mut mpsc::Receiver<String>,
        idle: &mut Instant,
    ) -> Option<CloseFrame> {
        tokio::select! {
            () = self.cancellation.cancelled() => {
                Some(Self::close(1001, "The server is stopping."))
            }
            () = sleep_until(*idle) => {
                Some(Self::close(1001, "The socket was idle for five minutes."))
            }
            received = socket.recv() => {
                *idle = Instant::now() + Self::IDLE;
                self.forward_received(received, input).await
            }
            output = outputs.recv() => {
                *idle = Instant::now() + Self::IDLE;
                self.outgoing(socket, input, output).await
            }
            peer = peers.recv() => Self::forward_peer(socket, peer).await,
        }
    }

    async fn forward_received(
        &self,
        received: Option<Result<Message, axum::Error>>,
        input: &mpsc::Sender<SocketInput>,
    ) -> Option<CloseFrame> {
        let text = self.incoming(received);
        Self::send_incoming(text, input).await
    }

    async fn send_incoming(
        text: Result<Option<String>, CloseFrame>,
        input: &mpsc::Sender<SocketInput>,
    ) -> Option<CloseFrame> {
        match text {
            Ok(None) => None,
            Err(close) => Some(close),
            Ok(Some(text)) => Self::send_message(input, text).await,
        }
    }

    async fn send_message(input: &mpsc::Sender<SocketInput>, text: String) -> Option<CloseFrame> {
        let sent = input.send(SocketInput::Message(text)).await;
        sent.is_err()
            .then(|| Self::close(1011, "The socket handler stopped."))
    }

    async fn forward_peer(socket: &mut WebSocket, peer: Option<String>) -> Option<CloseFrame> {
        match peer {
            None => Some(Self::close(1013, "The socket fell too far behind.")),
            Some(data) => Self::send_text(socket, data).await,
        }
    }

    async fn send_text(socket: &mut WebSocket, data: String) -> Option<CloseFrame> {
        let sent = socket.send(Message::Text(data.into())).await;
        sent.is_err()
            .then(|| Self::close(1001, "The browser went away."))
    }

    fn incoming(
        &self,
        received: Option<Result<Message, axum::Error>>,
    ) -> Result<Option<String>, CloseFrame> {
        let message = Self::received_message(received)?;
        let text = Self::message_text(message)?;
        text.map(|text| self.checked_text(text)).transpose()
    }

    fn received_message(
        received: Option<Result<Message, axum::Error>>,
    ) -> Result<Message, CloseFrame> {
        match received {
            Some(Ok(message)) => Ok(message),
            Some(Err(_)) => Err(Self::close(1009, "The message is larger than 64 KiB.")),
            None => Err(Self::close(1000, "")),
        }
    }

    fn message_text(message: Message) -> Result<Option<String>, CloseFrame> {
        match message {
            Message::Text(text) => Ok(Some(text.to_string())),
            Message::Binary(_) => Err(Self::close(1003, "Send JSON text, not binary data.")),
            Message::Close(_) => Err(Self::close(1000, "")),
            Message::Ping(_) | Message::Pong(_) => Ok(None),
        }
    }

    fn checked_text(&self, text: String) -> Result<String, CloseFrame> {
        Self::check_message_size(&text)?;
        let value = serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|_| Self::close(1007, "The message is not JSON."))?;
        self.check_message_fits(&value)?;
        Ok(text)
    }

    fn check_message_size(text: &str) -> Result<(), CloseFrame> {
        if text.len() > Self::MAX_MESSAGE_BYTES {
            return Err(Self::close(1009, "The message is larger than 64 KiB."));
        }
        Ok(())
    }

    fn check_message_fits(&self, value: &serde_json::Value) -> Result<(), CloseFrame> {
        let models = self.handler.models();
        self.mapping.parameters.iter().try_for_each(|parameter| {
            models
                .check(&parameter.shape, value, "message")
                .map_err(|problem| {
                    Self::close(1007, &format!("The message does not fit: {problem}"))
                })
        })
    }

    async fn outgoing(
        &self,
        socket: &mut WebSocket,
        input: &mpsc::Sender<SocketInput>,
        output: Option<SocketOutput>,
    ) -> Option<CloseFrame> {
        match output {
            None => Some(Self::close(1011, "The socket handler stopped.")),
            Some(SocketOutput::Send { to: Some(id), data }) if id != self.seat.id => {
                self.seat.hub.deliver(&self.seat.route, &id, data);
                None
            }
            Some(SocketOutput::Send { data, .. }) => Self::send_text(socket, data).await,
            Some(SocketOutput::AskSessions) => self.send_sessions(input).await,
            Some(SocketOutput::Close { code }) => Some(Self::close(code, "")),
        }
    }

    async fn send_sessions(&self, input: &mpsc::Sender<SocketInput>) -> Option<CloseFrame> {
        let ids = self.seat.hub.ids(&self.seat.route);
        let asked = input.send(SocketInput::Sessions(ids)).await;
        asked
            .is_err()
            .then(|| Self::close(1011, "The socket handler stopped."))
    }

    fn close(code: u16, reason: &str) -> CloseFrame {
        let mut end = reason.len().min(Self::MAX_REASON_BYTES);
        while !reason.is_char_boundary(end) {
            end -= 1;
        }
        CloseFrame {
            code,
            reason: reason[..end].to_owned().into(),
        }
    }
}
