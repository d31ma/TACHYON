use super::host::{copy_tree, native_io};
use crate::{Failure, WebCompiler};
use std::fs;
use std::path::Path;

pub(super) struct NativeWebStage;

impl NativeWebStage {
    const PREFIX: &'static str = ".tachyon-native-web-";
    const PUBLISHED_WEB: &'static str = "web";

    pub(super) fn create(output_root: &Path) -> Result<tempfile::TempDir, Failure> {
        native_io(fs::create_dir_all(output_root), output_root)?;
        native_io(
            tempfile::Builder::new()
                .prefix(Self::PREFIX)
                .tempdir_in(output_root),
            output_root,
        )
    }

    pub(super) fn seed(stage: &Path, previous_app: &Path, incremental: bool) {
        let previous = previous_app.join(Self::PUBLISHED_WEB);
        let usable = Self::usable(previous_app, &previous, incremental);
        let _copied = usable.then(|| Self::copy(stage, &previous));
    }

    fn usable(previous_app: &Path, previous: &Path, incremental: bool) -> bool {
        [
            incremental,
            Self::is_regular_directory(previous_app),
            Self::is_regular_directory(previous),
            WebCompiler::has_previous_build(previous),
        ] == [true; 4]
    }

    fn is_regular_directory(path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok_and(|metadata| {
            [metadata.file_type().is_symlink(), metadata.is_dir()] == [false, true]
        })
    }

    fn copy(stage: &Path, previous: &Path) {
        if copy_tree(previous, stage).is_err() {
            Self::empty(stage);
        }
    }

    fn empty(stage: &Path) {
        let _removed = fs::remove_dir_all(stage);
        let _created = fs::create_dir_all(stage);
    }
}
