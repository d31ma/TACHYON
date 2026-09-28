#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeInsertion {
    pub(crate) at: usize,
    pub(crate) text: String,
}
