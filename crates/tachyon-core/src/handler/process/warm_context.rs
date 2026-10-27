use crate::handler::topic_bus::TopicOutbox;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

pub(super) struct WarmContext {
    pub(super) request_id: String,
    pub(super) direct: bool,
    pub(super) topics: Option<Arc<TopicOutbox>>,
    pub(super) deadline: Instant,
    pub(super) grace: Duration,
}
