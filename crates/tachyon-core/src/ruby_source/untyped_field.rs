use std::ops::Range;

pub(crate) struct UntypedField {
    pub(crate) name: String,
    pub(crate) span: Range<usize>,
}
