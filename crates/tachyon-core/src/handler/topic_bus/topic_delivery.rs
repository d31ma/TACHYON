use serde_json::Value;

pub(crate) struct TopicDelivery {
    pub(super) topic: String,
    pub(super) payload: Value,
    pub(super) origin: String,
    pub(super) hops: u64,
}
