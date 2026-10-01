use std::ops::Range;

pub(crate) struct ScriptImport {
    pub(crate) range: Range<usize>,
    pub(crate) specifier: String,
}
