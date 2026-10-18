use std::path::PathBuf;

pub(super) struct RustPageOwner {
    pub(super) service: String,
    pub(super) identity: PathBuf,
    pub(super) relative: PathBuf,
    pub(super) portable: String,
    pub(super) excluded: Vec<String>,
    pub(super) entries: String,
    pub(super) required: Vec<String>,
}
