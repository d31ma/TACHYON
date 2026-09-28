use super::{TopicHub, TopicState};
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub(in crate::server) struct TopicAdmission {
    pub(in crate::server) hub: TopicHub,
    pub(in crate::server) name: String,
    pub(in crate::server) topic: Arc<TopicState>,
}

impl Drop for TopicAdmission {
    fn drop(&mut self) {
        self.release();
    }
}

impl TopicAdmission {
    fn release(&self) {
        let mut topics = self.hub.lock_topics();
        self.hub.subscribers.fetch_sub(1, Ordering::AcqRel);
        let previous = self.topic.subscribers.fetch_sub(1, Ordering::AcqRel);
        if previous == 1
            && topics
                .get(&self.name)
                .is_some_and(|current| Arc::ptr_eq(current, &self.topic))
        {
            topics.remove(&self.name);
            self.topic.cancellation.cancel();
        }
    }
}
