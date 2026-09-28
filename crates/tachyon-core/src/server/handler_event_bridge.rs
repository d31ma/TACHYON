use super::{Dispatch, DispatchRoute, EventStream, InvocationLog, ServerResponse};
use crate::Failure;
use crate::handler::{HandlerCancellation, HandlerEvent, HandlerSource, HandlerSupervisor};
use axum::body::Body;
use axum::http::Response;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tachyon_contracts::HandlerRequest;
use tokio::io::DuplexStream;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub(in crate::server) struct HandlerEventBridge;

impl HandlerEventBridge {
    const END_FRAME: &'static [u8] = b"event: end\ndata: {}\n\n";

    pub(in crate::server) fn start(
        state: &Dispatch,
        entry: &DispatchRoute,
        request: HandlerRequest,
    ) -> Response<Body> {
        let (sender, receiver) = mpsc::channel::<HandlerEvent>(Dispatch::STREAM_EVENT_QUEUE);
        let (completion_sender, completion_receiver) = oneshot::channel::<bool>();
        let request_id = request.request_id.clone();
        let producer = Self::produce(
            Arc::clone(&state.supervisor),
            entry.handler.clone(),
            request,
            sender,
            completion_sender,
            state.cancellation.clone(),
        );
        let _accepted = state.producers.spawn(producer);

        let (writer, reader) = tokio::io::duplex(Dispatch::SSE_BRIDGE_BYTES);
        let bridge = Self::bridge(
            writer,
            receiver,
            completion_receiver,
            request_id.clone(),
            state.cancellation.clone(),
        );
        let _accepted = state.producers.spawn(bridge);

        let mut response = EventStream::response(reader, "Cannot open the stream.");
        ServerResponse::insert_request_id(&mut response, &request_id);
        response
    }

    async fn produce(
        supervisor: Arc<HandlerSupervisor>,
        handler: HandlerSource,
        request: HandlerRequest,
        sender: mpsc::Sender<HandlerEvent>,
        completion_sender: oneshot::Sender<bool>,
        cancellation: CancellationToken,
    ) {
        let handler_cancellation = HandlerCancellation::default();
        let invocation = supervisor.invoke_streaming_cancellable(
            &handler,
            &request,
            sender,
            &handler_cancellation,
        );
        tokio::pin!(invocation);
        let failure =
            Self::failure_unless_stopped(invocation, &handler_cancellation, &cancellation).await;
        Self::log_unless_stopped(failure.as_ref(), &request, &handler, &cancellation);
        let _sent = completion_sender.send(failure.is_none());
    }

    async fn failure_unless_stopped<F>(
        invocation: Pin<&mut F>,
        handler_cancellation: &HandlerCancellation,
        cancellation: &CancellationToken,
    ) -> Option<Failure>
    where
        F: Future<Output = Result<(), Failure>>,
    {
        tokio::select! {
            result = invocation => result.err(),
            () = cancellation.cancelled() => {
                handler_cancellation.cancel();
                None
            }
        }
    }

    fn log_unless_stopped(
        failure: Option<&Failure>,
        request: &HandlerRequest,
        handler: &HandlerSource,
        cancellation: &CancellationToken,
    ) {
        if let Some(failure) = failure
            && !cancellation.is_cancelled()
        {
            InvocationLog::log_failure(
                "handler.stream",
                &request.request_id,
                InvocationLog::runtime_family(handler),
                failure,
            );
        }
    }

    async fn bridge(
        mut writer: DuplexStream,
        mut receiver: mpsc::Receiver<HandlerEvent>,
        mut completion_receiver: oneshot::Receiver<bool>,
        request_id: String,
        cancellation: CancellationToken,
    ) {
        let mut open = true;
        while open {
            open = Self::bridge_next(
                &mut writer,
                &mut receiver,
                &mut completion_receiver,
                &request_id,
                &cancellation,
            )
            .await;
        }
    }

    async fn bridge_next(
        writer: &mut DuplexStream,
        receiver: &mut mpsc::Receiver<HandlerEvent>,
        completion_receiver: &mut oneshot::Receiver<bool>,
        request_id: &str,
        cancellation: &CancellationToken,
    ) -> bool {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => false,
            completion = &mut *completion_receiver => {
                Self::drain(writer, receiver, completion, request_id, cancellation).await;
                false
            }
            event = receiver.recv() => {
                Self::forward_event(writer, event, completion_receiver, request_id, cancellation).await
            }
        }
    }

    async fn forward_event(
        writer: &mut DuplexStream,
        event: Option<HandlerEvent>,
        completion_receiver: &mut oneshot::Receiver<bool>,
        request_id: &str,
        cancellation: &CancellationToken,
    ) -> bool {
        match event {
            None => {
                let completion = completion_receiver.await;
                Self::finish(writer, completion, request_id, cancellation).await;
                false
            }
            Some(event) => Self::write_event(writer, &event, cancellation).await,
        }
    }

    async fn drain(
        writer: &mut DuplexStream,
        receiver: &mut mpsc::Receiver<HandlerEvent>,
        completion: Result<bool, oneshot::error::RecvError>,
        request_id: &str,
        cancellation: &CancellationToken,
    ) {
        if Self::write_remaining(writer, receiver, cancellation).await {
            Self::finish(writer, completion, request_id, cancellation).await;
        }
    }

    async fn write_remaining(
        writer: &mut DuplexStream,
        receiver: &mut mpsc::Receiver<HandlerEvent>,
        cancellation: &CancellationToken,
    ) -> bool {
        let mut outcome = None;
        while outcome.is_none() {
            outcome = Self::write_next_remaining(writer, receiver, cancellation).await;
        }
        outcome.unwrap_or(false)
    }

    async fn write_next_remaining(
        writer: &mut DuplexStream,
        receiver: &mut mpsc::Receiver<HandlerEvent>,
        cancellation: &CancellationToken,
    ) -> Option<bool> {
        match receiver.recv().await {
            None => Some(true),
            Some(event) => {
                let written = Self::write_event(writer, &event, cancellation).await;
                (!written).then_some(false)
            }
        }
    }

    async fn finish(
        writer: &mut DuplexStream,
        completion: Result<bool, oneshot::error::RecvError>,
        request_id: &str,
        cancellation: &CancellationToken,
    ) {
        let frame = Self::final_frame(completion, request_id);
        let _written = EventStream::write(writer, &frame, cancellation).await;
    }

    fn final_frame(
        completion: Result<bool, oneshot::error::RecvError>,
        request_id: &str,
    ) -> Vec<u8> {
        if completion.is_ok_and(|completed| completed) {
            return Self::END_FRAME.to_vec();
        }
        let data = serde_json::json!({
            "code": "TY2107",
            "message": "The handler stream ended unexpectedly.",
            "request_id": request_id,
        });
        format!("event: error\ndata: {data}\n\n").into_bytes()
    }

    async fn write_event(
        writer: &mut DuplexStream,
        event: &HandlerEvent,
        cancellation: &CancellationToken,
    ) -> bool {
        let frame = format!("data: {}\n\n", event.data);
        EventStream::write(writer, frame.as_bytes(), cancellation).await
    }
}
