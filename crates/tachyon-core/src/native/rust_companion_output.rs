use super::rust_native_libraries::RustNativeLibraries;
use std::path::PathBuf;

pub(super) struct RustCompanionOutput {
    pub(super) library: PathBuf,
    pub(super) executables: Vec<PathBuf>,
    pub(super) native_libraries: RustNativeLibraries,
}
