#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockKind {
    Type,
    Member,
    Value,
    Skip,
}
