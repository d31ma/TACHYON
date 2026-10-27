use super::{
    HandlerCancellation, HandlerEvent, ProcessFailure, StreamEvents, StreamReadOutcome,
    WarmAnswer, WarmContext, WarmOutput, WarmProcess, WarmStop, WarmStreamEnd,
};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};

pub(super) struct WarmStream<'a> {
    pub(super) process: WarmProcess,
    pub(super) context: WarmContext,
    pub(super) events: StreamEvents,
    pub(super) subscriber: mpsc::Sender<HandlerEvent>,
    pub(super) cancellation: &'a HandlerCancellation,
}

impl WarmStream<'_> {
    pub(super) async fn run(mut self, request_bytes: &[u8]) -> WarmAnswer<()> {
        self.process.begin_request();
        let cancellation = self.cancellation;
        let deadline = self.context.deadline;
        let subscriber = self.subscriber.clone();
        let streamed = self.stream(request_bytes);
        let bounded = Self::bounded(cancellation, deadline, &subscriber, streamed).await;
        let outcome = bounded.and_then(|outcome| outcome);
        self.settle(outcome).await
    }

    async fn bounded(
        cancellation: &HandlerCancellation,
        deadline: Instant,
        subscriber: &mpsc::Sender<HandlerEvent>,
        work: impl Future<Output = Result<StreamReadOutcome, WarmStop>>,
    ) -> Result<Result<StreamReadOutcome, WarmStop>, WarmStop> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(WarmStop::Cancelled),
            () = sleep_until(deadline) => Err(WarmStop::TimedOut),
            () = subscriber.closed() => Ok(Ok(StreamReadOutcome::SubscriberClosed)),
            outcome = work => Ok(outcome),
        }
    }

    async fn stream(&mut self, request_bytes: &[u8]) -> Result<StreamReadOutcome, WarmStop> {
        let sent = self.process.send(request_bytes).await;
        sent.map_err(WarmStop::Broken)?;
        let mut finished = None;
        while finished.is_none() {
            finished = self.forward_next().await?;
        }
        Ok(finished.unwrap_or(StreamReadOutcome::Complete))
    }

    async fn forward_next(&mut self) -> Result<Option<StreamReadOutcome>, WarmStop> {
        let output = self.process.next_output().await;
        let frame = Self::frame_of(output)?;
        self.forward(&frame).await
    }

    fn frame_of(output: WarmOutput) -> Result<Vec<u8>, WarmStop> {
        match output {
            WarmOutput::Frame(payload) => Ok(payload),
            WarmOutput::Ended => Err(WarmStop::Ended),
            WarmOutput::Oversized => Err(WarmStop::Broken(ProcessFailure::stream(
                "A streamed frame exceeds the 16 MiB protocol limit.",
            ))),
            WarmOutput::Truncated => Err(WarmStop::Broken(ProcessFailure::stream(
                "A streamed frame ended before its payload was complete.",
            ))),
        }
    }

    async fn forward(&mut self, frame: &[u8]) -> Result<Option<StreamReadOutcome>, WarmStop> {
        if StreamEvents::ends(frame, &self.context.request_id) {
            return Ok(Some(StreamReadOutcome::Complete));
        }
        self.events.forward(frame).await.map_err(WarmStop::Broken)
    }

    async fn settle(self, outcome: Result<StreamReadOutcome, WarmStop>) -> WarmAnswer<()> {
        match outcome {
            Ok(StreamReadOutcome::Complete) => WarmStreamEnd::complete(self.process).await,
            Ok(StreamReadOutcome::SubscriberClosed) => {
                WarmStreamEnd::abandoned(self.process).await
            }
            Err(stop) => WarmStreamEnd::stopped(stop, self.process, &self.context).await,
        }
    }
}
