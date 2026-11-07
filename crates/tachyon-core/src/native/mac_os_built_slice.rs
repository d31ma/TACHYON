use std::path::PathBuf;

pub(super) struct MacOsBuiltSlice {
    pub(super) executable: PathBuf,
    pub(super) helpers: Vec<PathBuf>,
    pub(super) swift_version: String,
}
