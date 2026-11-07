use std::ops::Range;

pub(crate) struct NeededField {
    pub(crate) name: String,
    pub(crate) type_name: String,
    pub(crate) span: Range<usize>,
}
