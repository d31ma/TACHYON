#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StreamReadOutcome {
    Complete,
    SubscriberClosed,
}
