use super::host::native_io;
use crate::Failure;
use std::fs;
use std::path::Path;

pub(super) struct NativeWebStage;

impl NativeWebStage {
    const PREFIX: &'static str = ".tachyon-native-web-";

    pub(super) fn create(output_root: &Path) -> Result<tempfile::TempDir, Failure> {
        native_io(fs::create_dir_all(output_root), output_root)?;
        native_io(
            tempfile::Builder::new()
                .prefix(Self::PREFIX)
                .tempdir_in(output_root),
            output_root,
        )
    }
}
