use super::{
    HandlerCancellation, WarmAnswer, WarmContext, WarmOutput, WarmProcess, WarmResponse, WarmStop,
};
use crate::handler::frame::{FramePrefix, HandlerFrame};
use crate::handler::resources::PipedResources;
use futures_util::future::OptionFuture;
use serde_json::Value;
use tachyon_contracts::{HandlerRequest, HandlerResponse};
use tokio::time::{Instant, sleep_until};

pub(super) struct WarmExchange<'a> {
    pub(super) process: WarmProcess,
    pub(super) context: WarmContext,
    pub(super) request: HandlerRequest,
    pub(super) resources: Option<PipedResources>,
    pub(super) style: FramePrefix,
    pub(super) cancellation: &'a HandlerCancellation,
}

impl WarmExchange<'_> {
    pub(super) async fn run(mut self) -> WarmAnswer<HandlerResponse> {
        self.process.begin_request();
        let cancellation = self.cancellation;
        let deadline = self.context.deadline;
        let bounded = Self::bounded(cancellation, deadline, self.exchange()).await;
        let reply = bounded.and_then(|reply| reply);
        self.settle(reply).await
    }

    async fn bounded<T>(
        cancellation: &HandlerCancellation,
        deadline: Instant,
        work: impl Future<Output = T>,
    ) -> Result<T, WarmStop> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(WarmStop::Cancelled),
            () = sleep_until(deadline) => Err(WarmStop::TimedOut),
            value = work => Ok(value),
        }
    }

    async fn exchange(&mut self) -> Result<Vec<u8>, WarmStop> {
        self.process.until_ready().await?;
        let request_bytes = self.context.request_frame(&self.request, self.style)?;
        let sent = self.process.send(&request_bytes).await;
        sent.map_err(WarmStop::Broken)?;
        self.final_frame().await
    }

    async fn final_frame(&mut self) -> Result<Vec<u8>, WarmStop> {
        let mut frame = self.next_frame().await?;
        while let Some(command) = self.resource_command(&frame) {
            self.answer_resource(&command).await?;
            frame = self.next_frame().await?;
        }
        Ok(frame)
    }

    async fn next_frame(&mut self) -> Result<Vec<u8>, WarmStop> {
        let output = self.process.next_output().await;
        Self::frame_of(output)
    }

    fn frame_of(output: WarmOutput) -> Result<Vec<u8>, WarmStop> {
        match output {
            WarmOutput::Frame(payload) => Ok(payload),
            WarmOutput::Ended => Err(WarmStop::Ended),
            WarmOutput::Oversized => Err(WarmStop::Broken(HandlerFrame::protocol_failure(
                2103,
                "Handler response frame exceeds the 16 MiB protocol limit.",
            ))),
            WarmOutput::Truncated => Err(WarmStop::Broken(HandlerFrame::protocol_failure(
                2102,
                "Handler stdout ended before the declared frame was complete.",
            ))),
        }
    }

    fn resource_command(&self, frame: &[u8]) -> Option<Value> {
        self.resources
            .as_ref()
            .and_then(|_| PipedResources::command(frame))
    }

    async fn answer_resource(&mut self, command: &Value) -> Result<(), WarmStop> {
        let answering = self.resources.as_ref().map(|piped| piped.answer(command));
        let answer = OptionFuture::from(answering).await.unwrap_or_default();
        let framed = [self.style.encode(answer.len()), answer].concat();
        self.process.send(&framed).await.map_err(WarmStop::Broken)
    }

    async fn settle(self, reply: Result<Vec<u8>, WarmStop>) -> WarmAnswer<HandlerResponse> {
        match reply {
            Ok(frame) => WarmResponse::read(&frame, self.process, &self.context).await,
            Err(stop) => {
                let failure = stop.into_failure(self.process, &self.context).await;
                WarmAnswer::without_process(Err(failure))
            }
        }
    }
}
