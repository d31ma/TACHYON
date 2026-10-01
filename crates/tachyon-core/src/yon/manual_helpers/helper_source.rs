use std::ops::Range;

pub(in crate::yon) struct HelperSource {
    pub(in crate::yon) name: String,
    pub(in crate::yon) body: Range<usize>,
    pub(in crate::yon) declaration: usize,
    pub(in crate::yon) managed: bool,
    pub(in crate::yon) zero_argument: bool,
    pub(in crate::yon) implementations: Vec<Range<usize>>,
}
