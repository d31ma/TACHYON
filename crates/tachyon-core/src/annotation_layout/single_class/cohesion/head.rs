use std::ops::Range;

pub(super) struct Head {
    pub(super) at: usize,
    pub(super) name_at: usize,
    pub(super) body: Range<usize>,
    pub(super) resume: usize,
}
