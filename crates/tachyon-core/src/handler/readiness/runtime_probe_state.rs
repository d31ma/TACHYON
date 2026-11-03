#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeProbeState {
    Ready,
    Missing,
    Incomplete,
}
