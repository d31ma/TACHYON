#![allow(clippy::expect_used)]

use std::path::Path;

pub struct FixtureFolder;

impl FixtureFolder {
    pub fn copy(source: &Path, target: &Path) {
        std::fs::create_dir_all(target).expect("folder");
        for entry in std::fs::read_dir(source).expect("fixture folder") {
            let path = entry.expect("fixture entry").path();
            let name = path.file_name().expect("file name");
            if path.is_dir() {
                Self::copy(&path, &target.join(name));
            } else {
                std::fs::copy(&path, target.join(name)).expect("copy fixture");
            }
        }
    }
}
