#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum Layout {
    Braced,
    Indented,
    Ended,
}
