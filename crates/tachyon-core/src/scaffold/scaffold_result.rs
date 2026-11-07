use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaffoldResult {
    pub(crate) root: PathBuf,
    pub(crate) app_name: String,
}

impl ScaffoldResult {
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn app_name(&self) -> &str {
        &self.app_name
    }
}
