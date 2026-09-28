use std::path::Path;

pub(super) struct QuerySource<'a> {
    pub(super) path: &'a Path,
    pub(super) extension: &'a str,
    pub(super) text: &'a str,
    pub(super) code: String,
}
