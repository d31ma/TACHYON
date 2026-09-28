use crate::handler::{HandlerSupervisor, TopicDeliveries};
use crate::server::WatchTask;
use std::sync::Arc;

pub(in crate::server) struct BackgroundPlan {
    pub(in crate::server) supervisor: Arc<HandlerSupervisor>,
    pub(in crate::server) workers: Vec<crate::project::ScheduledWorker>,
    pub(in crate::server) watcher: Option<WatchTask>,
    pub(in crate::server) topics: TopicDeliveries,
}
