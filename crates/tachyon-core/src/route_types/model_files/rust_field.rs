use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RustField {
    pub name: String,
    pub type_text: String,
    pub name_range: Range<usize>,
    pub type_range: Range<usize>,
}

impl RustField {
    pub(crate) const RAW_PREFIX: &'static str = "r#";

    pub(crate) fn key(&self) -> &str {
        self.name
            .strip_prefix(Self::RAW_PREFIX)
            .unwrap_or(&self.name)
    }
}
