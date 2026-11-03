use tokio::task::JoinSet;

pub(super) struct ProducerTaskState {
    pub(super) accepting: bool,
    pub(super) tasks: JoinSet<()>,
}
