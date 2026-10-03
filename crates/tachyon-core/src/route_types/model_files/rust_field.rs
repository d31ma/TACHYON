use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RustField {
    pub name: String,
    pub type_text: String,
    pub name_range: Range<usize>,
    pub type_range: Range<usize>,
}
