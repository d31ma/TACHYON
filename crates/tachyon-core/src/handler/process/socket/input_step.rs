#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InputStep {
    Open,
    Ended,
    Stopped,
}
