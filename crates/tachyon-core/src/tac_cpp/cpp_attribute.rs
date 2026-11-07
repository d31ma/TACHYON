use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CppAttribute {
    pub(crate) name: String,
    pub(crate) arguments: String,
    pub(crate) at: usize,
    pub(crate) specifier: Range<usize>,
}

impl CppAttribute {
    pub(crate) fn has_arguments(&self) -> bool {
        !self.arguments.trim().is_empty()
    }
}
