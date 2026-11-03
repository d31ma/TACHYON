#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ModelBlock {
    pub(crate) name: String,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl ModelBlock {
    pub(crate) fn new(name: &str, start: usize, end: usize) -> Self {
        Self {
            name: name.to_owned(),
            start,
            end,
        }
    }

    pub(crate) fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}
