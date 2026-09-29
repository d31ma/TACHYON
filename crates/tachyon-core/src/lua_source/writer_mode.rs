#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WriterMode {
    Top,
    Table,
    Parameters,
    Body,
}
