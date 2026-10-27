use super::direct_response::DirectResponse;
use super::{WarmAnswer, WarmContext, WarmProcess};
use crate::Failure;
use crate::handler::frame::{FramePrefix, HandlerFrame};
use tachyon_contracts::HandlerResponse;

pub(super) struct WarmResponse;

impl WarmResponse {
    pub(super) async fn read(
        frame: &[u8],
        process: WarmProcess,
        context: &WarmContext,
    ) -> WarmAnswer<HandlerResponse> {
        let parsed = Self::parse(frame, context).await;
        let stderr = process.take_stderr();
        let checked = stderr.check_diagnostics().and(parsed);
        stderr.surface_events(&context.request_id);
        WarmAnswer::after(checked, process).await
    }

    async fn parse(frame: &[u8], context: &WarmContext) -> Result<HandlerResponse, Failure> {
        if context.direct {
            let topics = context.topics.clone();
            return DirectResponse::parse(frame, &context.request_id, topics).await;
        }
        let framed = [FramePrefix::Binary.encode(frame.len()), frame.to_vec()].concat();
        HandlerFrame::response(&framed, &context.request_id)
    }
}
