#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UnusedMember {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) start: usize,
    pub(crate) field: bool,
}
