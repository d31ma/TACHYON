use std::path::Path;

pub(super) struct HeldService<'a> {
    pub(super) class: &'a str,
    pub(super) language: &'a str,
    pub(super) path: &'a Path,
}
