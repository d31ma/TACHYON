use std::ops::Range;

pub(super) struct JavaPageType {
    pub(super) name: String,
    pub(super) span: Range<usize>,
}
