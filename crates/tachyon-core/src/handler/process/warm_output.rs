#[derive(Debug, Eq, PartialEq)]
pub(super) enum WarmOutput {
    Frame(Vec<u8>),
    Ended,
    Oversized,
    Truncated,
}
