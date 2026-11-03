#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerEvent {
    pub data: String,
}

impl HandlerEvent {
    pub(super) const MAX_EVENTS_PER_STREAM: usize = 100_000;
    pub(super) const MAX_BYTES: usize = 256 * 1024;
    pub(super) const MAX_TOTAL_BYTES_PER_STREAM: usize = 64 * 1024 * 1024;
}
