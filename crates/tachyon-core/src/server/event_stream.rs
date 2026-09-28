use super::{HeaderNames, ServerResponse};
use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONNECTION, CONTENT_TYPE};
use axum::http::{Request, Response, StatusCode};
use tokio::io::{AsyncWriteExt as _, DuplexStream};
use tokio_util::io::ReaderStream;
use tokio_util::sync::CancellationToken;

pub(in crate::server) struct EventStream;

impl EventStream {
    pub(in crate::server) async fn write(
        writer: &mut DuplexStream,
        bytes: &[u8],
        cancellation: &CancellationToken,
    ) -> bool {
        Self::write_unless_cancelled(writer, bytes, cancellation).await
    }

    pub(in crate::server) fn response(
        reader: DuplexStream,
        failure_message: &str,
    ) -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/event-stream; charset=utf-8")
            .header(CACHE_CONTROL, "no-store")
            .header(CONNECTION, "keep-alive")
            .header("x-accel-buffering", "no")
            .body(Body::from_stream(ReaderStream::new(reader)))
            .unwrap_or_else(|_| {
                ServerResponse::text(StatusCode::INTERNAL_SERVER_ERROR, failure_message)
            })
    }

    pub(in crate::server) fn topic_response(reader: DuplexStream) -> Response<Body> {
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/event-stream")
            .header(CACHE_CONTROL, "no-store")
            .header("x-accel-buffering", "no")
            .body(Body::from_stream(ReaderStream::new(reader)))
            .unwrap_or_else(|_| {
                ServerResponse::text(StatusCode::INTERNAL_SERVER_ERROR, "Cannot open the stream.")
            })
    }

    pub(in crate::server) fn last_event_sequence(request: &Request<Body>) -> Option<u64> {
        let value = request.headers().get(HeaderNames::LAST_EVENT_ID)?;
        value.to_str().ok()?.parse::<u64>().ok()
    }

    async fn write_unless_cancelled(
        writer: &mut DuplexStream,
        bytes: &[u8],
        cancellation: &CancellationToken,
    ) -> bool {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => false,
            result = writer.write_all(bytes) => result.is_ok(),
        }
    }
}
