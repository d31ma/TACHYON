use super::{
    Dispatch, EventStream, ServerResponse, TopicAdmission, TopicFailure, TopicHub, TopicRecord,
    TopicReplay, TopicState,
};
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use std::sync::Arc;
use tokio::io::DuplexStream;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub(in crate::server) struct TopicStream;

impl TopicStream {
    const SLUG_SYMBOL: u8 = b'-';

    pub(in crate::server) fn open(
        state: &Dispatch,
        topic: &str,
        request: &Request<Body>,
    ) -> Response<Body> {
        let slug = Self::is_topic_slug(topic);
        Self::open_slug(slug, state, topic, request)
    }

    pub(in crate::server) fn requested_position(request: &Request<Body>) -> Option<u64> {
        EventStream::last_event_sequence(request)
            .map(|last_event_id| last_event_id.saturating_add(1))
            .or_else(|| Self::query_position(request))
    }

    pub(in crate::server) async fn close_stale_cursor(
        mut writer: DuplexStream,
        cancellation: CancellationToken,
    ) {
        let frame = TopicFailure::error_frame(
            "TY_TOPIC_CURSOR_STALE",
            "Topic cursor is no longer available.",
            "Close this subscription and reconnect without an explicit cursor.",
        );
        let _ = EventStream::write(&mut writer, frame.as_bytes(), &cancellation).await;
    }

    fn is_topic_slug(topic: &str) -> bool {
        !topic.is_empty()
            && topic.len() <= TopicHub::MAX_NAME_BYTES
            && topic.bytes().all(Self::slug_byte)
    }

    fn slug_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == Self::SLUG_SYMBOL
    }

    fn open_slug(
        slug: bool,
        state: &Dispatch,
        topic: &str,
        request: &Request<Body>,
    ) -> Response<Body> {
        if !slug {
            return ServerResponse::text(StatusCode::BAD_REQUEST, "Topic must be a bounded slug.");
        }
        Self::subscribe(state, topic, request)
    }

    fn subscribe(state: &Dispatch, topic: &str, request: &Request<Body>) -> Response<Body> {
        let requested = Self::requested_position(request);
        let subscription = state.topic_hub.subscribe(topic);
        Self::stream_subscription(subscription, requested, state)
    }

    fn stream_subscription(
        subscription: Result<(Arc<TopicState>, TopicAdmission), &'static str>,
        requested: Option<u64>,
        state: &Dispatch,
    ) -> Response<Body> {
        match subscription {
            Ok((topic_state, admission)) => {
                Self::start_stream(&topic_state, admission, requested, state)
            }
            Err(message) => ServerResponse::text(StatusCode::SERVICE_UNAVAILABLE, message),
        }
    }

    fn start_stream(
        topic_state: &TopicState,
        admission: TopicAdmission,
        requested: Option<u64>,
        state: &Dispatch,
    ) -> Response<Body> {
        let receiver = topic_state.sender.subscribe();
        let replay = topic_state.replay_snapshot(requested);
        let (writer, reader) = tokio::io::duplex(TopicHub::STREAM_BUFFER_BYTES);
        let producer = Self::send_frames(
            writer,
            receiver,
            replay,
            admission,
            state.cancellation.clone(),
        );
        let _accepted = state.producers.spawn(producer);
        EventStream::topic_response(reader)
    }

    fn query_position(request: &Request<Body>) -> Option<u64> {
        let query = request.uri().query()?;
        let position = query
            .split('&')
            .find_map(|pair| pair.strip_prefix("position="))?;
        position.parse::<u64>().ok()
    }

    async fn send_frames(
        mut writer: DuplexStream,
        mut receiver: broadcast::Receiver<TopicRecord>,
        replay: TopicReplay,
        admission: TopicAdmission,
        cancellation: CancellationToken,
    ) {
        let _admission = admission;
        if replay.cursor_gap {
            Self::close_stale_cursor(writer, cancellation).await;
            return;
        }
        Self::send_replay_then_live(&mut writer, &mut receiver, replay, &cancellation).await;
    }

    async fn send_replay_then_live(
        writer: &mut DuplexStream,
        receiver: &mut broadcast::Receiver<TopicRecord>,
        replay: TopicReplay,
        cancellation: &CancellationToken,
    ) {
        let next = Self::send_replayed(writer, replay, cancellation).await;
        Self::send_live(writer, receiver, next, cancellation).await;
    }

    async fn send_replayed(
        writer: &mut DuplexStream,
        replay: TopicReplay,
        cancellation: &CancellationToken,
    ) -> Option<u64> {
        let mut next = Some(replay.resume);
        for (position, frame, terminal) in replay.frames {
            next =
                Self::send_replayed_frame(writer, next?, (position, frame, terminal), cancellation)
                    .await;
        }
        next
    }

    async fn send_replayed_frame(
        writer: &mut DuplexStream,
        next: u64,
        record: TopicRecord,
        cancellation: &CancellationToken,
    ) -> Option<u64> {
        let skip = Self::replay_skips(&record, next);
        Self::write_unless_skipped(writer, skip, next, record, cancellation).await
    }

    fn replay_skips(record: &TopicRecord, next: u64) -> bool {
        record.0 < next && !record.2
    }

    async fn send_live(
        writer: &mut DuplexStream,
        receiver: &mut broadcast::Receiver<TopicRecord>,
        next: Option<u64>,
        cancellation: &CancellationToken,
    ) {
        let mut cursor = next;
        while let Some(next) = cursor {
            cursor = Self::send_next_live(writer, receiver, next, cancellation).await;
        }
    }

    async fn send_next_live(
        writer: &mut DuplexStream,
        receiver: &mut broadcast::Receiver<TopicRecord>,
        next: u64,
        cancellation: &CancellationToken,
    ) -> Option<u64> {
        let message = Self::next_record(receiver, cancellation).await;
        Self::handle_record(writer, message, next, cancellation).await
    }

    async fn next_record(
        receiver: &mut broadcast::Receiver<TopicRecord>,
        cancellation: &CancellationToken,
    ) -> Option<Result<TopicRecord, broadcast::error::RecvError>> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            result = receiver.recv() => Some(result),
        }
    }

    async fn handle_record(
        writer: &mut DuplexStream,
        message: Option<Result<TopicRecord, broadcast::error::RecvError>>,
        next: u64,
        cancellation: &CancellationToken,
    ) -> Option<u64> {
        match message {
            None | Some(Err(broadcast::error::RecvError::Closed)) => None,
            Some(Err(broadcast::error::RecvError::Lagged(_))) => {
                Self::report_slow_subscriber(writer, cancellation).await;
                None
            }
            Some(Ok(record)) => Self::send_live_record(writer, record, next, cancellation).await,
        }
    }

    async fn send_live_record(
        writer: &mut DuplexStream,
        record: TopicRecord,
        next: u64,
        cancellation: &CancellationToken,
    ) -> Option<u64> {
        let skip = record.0 < next;
        Self::write_unless_skipped(writer, skip, next, record, cancellation).await
    }

    async fn write_unless_skipped(
        writer: &mut DuplexStream,
        skip: bool,
        next: u64,
        record: TopicRecord,
        cancellation: &CancellationToken,
    ) -> Option<u64> {
        if skip {
            return Some(next);
        }
        Self::write_record(writer, record, cancellation).await
    }

    async fn write_record(
        writer: &mut DuplexStream,
        (position, frame, terminal): TopicRecord,
        cancellation: &CancellationToken,
    ) -> Option<u64> {
        let written = EventStream::write(writer, frame.as_bytes(), cancellation).await;
        Self::next_after(written, terminal, position)
    }

    fn next_after(written: bool, terminal: bool, position: u64) -> Option<u64> {
        match (written, terminal) {
            (true, false) => Some(position.saturating_add(1)),
            _ => None,
        }
    }

    async fn report_slow_subscriber(writer: &mut DuplexStream, cancellation: &CancellationToken) {
        let frame = TopicFailure::error_frame(
            "TY_TOPIC_SLOW_SUBSCRIBER",
            "Topic subscriber was too slow.",
            "Close this subscription and reconnect without an explicit cursor.",
        );
        let _ = EventStream::write(writer, frame.as_bytes(), cancellation).await;
    }
}
