use std::ops::Range;

pub(super) struct Container {
    pub(super) range: Range<usize>,
    pub(super) depth: usize,
    pub(super) public_members: bool,
}
