use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) struct ModuleInput<'a> {
    pub(super) entry: &'a Path,
    pub(super) source: &'a str,
    pub(super) paths: &'a [PathBuf],
    pub(super) environment: &'a str,
}
