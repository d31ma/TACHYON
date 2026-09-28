use super::ViewKind;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ViewSource {
    pub(super) source_path: String,
    pub(super) absolute_source_path: PathBuf,
    pub(super) view_kind: ViewKind,
    pub(super) bytes: Vec<u8>,
}
