use super::{HandlerSupervisor, WarmStop};
use crate::handler::frame::{FramePrefix, HandlerFrame};
use crate::handler::topic_bus::TopicOutbox;
use std::sync::Arc;
use std::time::Duration;
use tachyon_contracts::HandlerRequest;
use tokio::time::Instant;

pub(super) struct WarmContext {
    pub(super) request_id: String,
    pub(super) direct: bool,
    pub(super) topics: Option<Arc<TopicOutbox>>,
    pub(super) deadline: Instant,
    pub(super) grace: Duration,
}

impl WarmContext {
    /// The request frame, with `deadline_ms` measured now: after the warm process said it is ready.
    pub(super) fn request_frame(
        &self,
        request: &HandlerRequest,
        style: FramePrefix,
    ) -> Result<Vec<u8>, WarmStop> {
        let timed = HandlerSupervisor::timed_request(request, self.deadline, self.grace);
        let frame = timed.and_then(|timed| HandlerFrame::request(&timed));
        frame
            .map(|frame| style.reframed(&frame))
            .map_err(WarmStop::Broken)
    }
}
