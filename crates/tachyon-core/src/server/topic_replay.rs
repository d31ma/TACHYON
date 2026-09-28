use super::TopicRecord;

pub(in crate::server) struct TopicReplay {
    pub(in crate::server) resume: u64,
    pub(in crate::server) frames: Vec<TopicRecord>,
    pub(in crate::server) cursor_gap: bool,
}
