#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublicMethod {
    pub(crate) name: String,
    pub(crate) line: usize,
}

impl PublicMethod {
    pub(crate) fn new(name: String, line: usize) -> Self {
        Self { name, line }
    }
}
