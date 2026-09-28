use super::ContentEncoding;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct PreviousOutput {
    root: PathBuf,
}

impl PreviousOutput {
    pub(super) fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub(super) fn reusable_variant(
        &self,
        file: &Path,
        bytes: &[u8],
        encoding: ContentEncoding,
    ) -> Option<Vec<u8>> {
        self.has_same_file(file, bytes)
            .then(|| fs::read(self.root.join(encoding.variant_path(file))).ok())
            .flatten()
    }

    fn has_same_file(&self, file: &Path, bytes: &[u8]) -> bool {
        let previous = self.root.join(file);
        Self::has_same_length(&previous, bytes)
            .then(|| fs::read(&previous).ok())
            .flatten()
            .is_some_and(|previous| previous == bytes)
    }

    fn has_same_length(previous: &Path, bytes: &[u8]) -> bool {
        fs::metadata(previous)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == bytes.len() as u64)
    }
}
