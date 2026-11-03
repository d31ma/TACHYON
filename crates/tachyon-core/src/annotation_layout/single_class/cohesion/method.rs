use std::ops::Range;

#[derive(Clone, Debug)]
pub(in super::super) struct Method {
    pub(in super::super) name: String,
    pub(in super::super) head: usize,
    pub(in super::super) public: bool,
    pub(in super::super) body: Range<usize>,
    pub(in super::super) receiver: String,
    pub(in super::super) nested: Vec<Range<usize>>,
}

impl Method {
    pub(in super::super) fn owns(&self, at: usize) -> bool {
        let nested = self.nested.iter().any(|range| range.contains(&at));
        self.body.contains(&at) & !nested
    }
}
